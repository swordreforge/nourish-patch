//! Pyramid metadata, filesystem I/O, LOD selection, and tile enumeration.
//!
//! ## Layout
//!
//! ```text
//! ~/.config/y5/wallpaper/<sha256(source)>.cache/
//!   index.json
//!   L0/    000_000.png
//!   L1/    000_000.png  001_000.png
//!   L2/    000_000.png  001_000.png  002_000.png  ...
//! ```
//!
//! `index.json` records source dimensions and every LOD's size / tile grid so
//! the renderer can reconstruct the pyramid without scanning directories.

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Simple 2-D rectangle — avoids depending on image::math which was removed
// in recent image crate versions.
// ---------------------------------------------------------------------------

/// A 2-D rectangle with f64 coordinates.
#[derive(Debug, Clone, Copy, Default)]
pub struct RectF64 {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl RectF64 {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum TileError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image decode error: {0}")]
    Image(#[from] image::ImageError),
    #[error("JSON (de)serialization: {0}")]
    Json(#[from] serde_json::Error),
    #[error("source image too small (minimum {min}×{min} px)")]
    TooSmall { min: u32 },
    #[error("invalid LOD index {0}")]
    InvalidLod(u8),
}

// ---------------------------------------------------------------------------
// Metadata types
// ---------------------------------------------------------------------------

/// One level-of-detail in the tile pyramid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelMeta {
    pub level: u8,
    /// Logical width of this LOD in pixels (source_w >> level).
    pub w: u32,
    /// Logical height of this LOD in pixels (source_h >> level).
    pub h: u32,
    /// Number of tile columns that cover this LOD.
    pub cols: u32,
    /// Number of tile rows that cover this LOD.
    pub rows: u32,
}

/// Full index describing a tile pyramid on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileIndex {
    /// Absolute path of the original source image.
    pub source: PathBuf,
    /// Source image width (pixels).
    pub source_w: u32,
    /// Source image height (pixels).
    pub source_h: u32,
    /// Tile edge length in pixels (uniform, currently always 512).
    pub tile_size: u32,
    /// Metadata for every LOD in the pyramid.
    pub levels: Vec<LevelMeta>,
}

impl TileIndex {
    /// Maximum number of LOD levels to generate (0 = coarsest, *levels*-1 = finest).
    pub fn max_level(&self) -> u8 {
        self.levels.len().saturating_sub(1) as u8
    }

    // ------------------------------------------------------------------
    // Factory: load from cache or generate from source
    // ------------------------------------------------------------------

    /// Load an existing tile pyramid from disk, or build one from the
    /// source image.  Returns the index when the pyramid is ready.
    pub fn load_or_generate(source: &Path) -> Result<Arc<Self>, TileError> {
        let cache_root = cache_dir(source);
        let index_path = cache_root.join("index.json");

        if index_path.exists() {
            let raw = std::fs::read_to_string(&index_path)?;
            let index: TileIndex = serde_json::from_str(&raw)?;
            return Ok(Arc::new(index));
        }

        let index = generate_pyramid(source, &cache_root)?;
        Ok(Arc::new(index))
    }

    // ------------------------------------------------------------------
    // LOD selection
    // ------------------------------------------------------------------

    /// Choose the LOD where one tile pixel ≈ one screen pixel.
    ///
    /// `zoom` is the current camera zoom (1.0 = full world fits), `screen_w`
    /// is the viewport width in physical pixels.
    pub fn select_lod(&self, zoom: f64, screen_w: f64) -> u8 {
        let max = self.max_level();
        let world_w = self.source_w as f64; // world-space pixel width

        // How many source pixels are visible across the screen width?
        // At zoom=1.0 the full image width is visible.
        let visible_ratio = 1.0_f64.min(1.0); // full image width at any zoom
                                             // (the viewport in world space = screen_w / zoom)
        let viewport_world = screen_w / zoom;
        let visible_frac = (viewport_world / world_w).clamp(0.0, 1.0);
        let visible_source_px = screen_w / visible_frac.max(0.001);

        let lod_f = (self.source_w as f64 / visible_source_px).log2().floor();
        (lod_f as u8).clamp(0, max)
    }

    // ------------------------------------------------------------------
    // Tile enumeration (world-space rect → tile coordinates)
    // ------------------------------------------------------------------

    /// World-space image bounds in the y5 coordinate model.
    /// The image occupies `world_w` × `world_h` world units at the origin.
    pub const WORLD_W: f64 = 10.0;
    pub const WORLD_H: f64 = 2.0;

    /// Compute the set of tile coordinates visible within `world_rect`.
    ///
    /// `world_rect` is the viewport rectangle in world coordinates (as produced
    /// by camera pan/zoom math).  Returns `(lod, col, row)` tuples.
    pub fn covering_tiles(
        &self,
        lod: u8,
        world_rect: &RectF64,
    ) -> Vec<(u8, u32, u32)> {
        let max = self.max_level();
        if lod > max {
            return vec![];
        }
        let lm = &self.levels[lod as usize];
        let ts = self.tile_size as f64;

        // Map viewport world rect to pixel coordinates at this LOD.
        let x0 = (world_rect.x / Self::WORLD_W * lm.w as f64).max(0.0);
        let y0 = (world_rect.y / Self::WORLD_H * lm.h as f64).max(0.0);
        let x1 = ((world_rect.x + world_rect.w) / Self::WORLD_W * lm.w as f64).min(lm.w as f64);
        let y1 = ((world_rect.y + world_rect.h) / Self::WORLD_H * lm.h as f64).min(lm.h as f64);

        let col_start = (x0 / ts).floor() as u32;
        let col_end = ((x1 / ts).ceil() as u32).min(lm.cols.saturating_sub(1));
        let row_start = (y0 / ts).floor() as u32;
        let row_end = ((y1 / ts).ceil() as u32).min(lm.rows.saturating_sub(1));

        let mut tiles = Vec::with_capacity(
            ((col_end - col_start + 1) * (row_end - row_start + 1)) as usize,
        );
        for row in row_start..=row_end {
            for col in col_start..=col_end {
                tiles.push((lod, col, row));
            }
        }
        tiles
    }

    /// Load the raw RGBA bytes of a single tile from disk.
    pub fn load_tile_bytes(&self, cache_root: &Path, lod: u8, col: u32, row: u32) -> Result<Vec<u8>, TileError> {
        let lm = self.level(lod)?;
        if col >= lm.cols || row >= lm.rows {
            return Err(TileError::InvalidLod(lod));
        }
        let tile_path = cache_root
            .join(format!("L{lod}"))
            .join(format!("{col:03}_{row:03}.png"));

        let img = image::open(&tile_path)?;
        Ok(img.into_rgba8().into_raw())
    }

    /// Get the pixel dimensions of a tile in the given LOD (always `tile_size`²
    /// except possibly edge tiles which are smaller).
    pub fn tile_dimensions(&self, lod: u8, col: u32, row: u32) -> (u32, u32) {
        let lm = self.level(lod).unwrap_or_else(|_| &self.levels[0]);
        let ts = self.tile_size;
        let tw = if col < lm.cols.saturating_sub(1) { ts } else { lm.w - col * ts };
        let th = if row < lm.rows.saturating_sub(1) { ts } else { lm.h - row * ts };
        (tw.max(1), th.max(1))
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn level(&self, lod: u8) -> Result<&LevelMeta, TileError> {
        self.levels.get(lod as usize).ok_or(TileError::InvalidLod(lod))
    }
}

// ---------------------------------------------------------------------------
// Pyramid generation
// ---------------------------------------------------------------------------

fn generate_pyramid(source: &Path, cache_root: &Path) -> Result<TileIndex, TileError> {
    let img = image::open(source)?;
    let (source_w, source_h) = (img.width(), img.height());

    if source_w < 512 || source_h < 512 {
        return Err(TileError::TooSmall { min: 512 });
    }

    let tile_size = 512u32;
    let mut levels = Vec::new();
    let max_level = compute_max_level(source_w, source_h);

    std::fs::create_dir_all(cache_root)?;

    for lod in 0..=max_level {
        let lod_w = (source_w as f64 / 2u32.pow(lod) as f64).ceil() as u32;
        let lod_h = (source_h as f64 / 2u32.pow(lod) as f64).ceil() as u32;

        let (clamped_w, clamped_h) = if lod == 0 {
            (tile_size.min(lod_w), tile_size.min(lod_h))
        } else {
            (lod_w, lod_h)
        };

        let cols = clamped_w.div_ceil(tile_size);
        let rows = clamped_h.div_ceil(tile_size);

        levels.push(LevelMeta { level: lod as u8, w: clamped_w, h: clamped_h, cols, rows });

        // Downscale the source to this LOD's size, then slice into tiles.
        let mut lod_img = image::DynamicImage::from(image::imageops::resize(
            &img,
            clamped_w.max(1),
            clamped_h.max(1),
            image::imageops::FilterType::Triangle,
        ));
        let lod_dir = cache_root.join(format!("L{lod}"));
        std::fs::create_dir_all(&lod_dir)?;

        for row in 0..rows {
            for col in 0..cols {
                let x = col * tile_size;
                let y = row * tile_size;
                let tw = tile_size.min(clamped_w.saturating_sub(x));
                let th = tile_size.min(clamped_h.saturating_sub(y));
                if tw == 0 || th == 0 {
                    continue;
                }
                let tile = lod_img.crop(x, y, tw, th);
                let tile_path = lod_dir.join(format!("{col:03}_{row:03}.png"));
                tile.save(&tile_path)?;
            }
        }
    }

    let index = TileIndex {
        source: source.canonicalize()?,
        source_w,
        source_h,
        tile_size,
        levels,
    };

    let index_json = serde_json::to_string_pretty(&index)?;
    std::fs::write(cache_root.join("index.json"), &index_json)?;

    Ok(index)
}

/// Compute the number of LOD levels so that the coarsest LOD fits in about one tile.
fn compute_max_level(source_w: u32, source_h: u32) -> u32 {
    let mut level = 0u32;
    let mut w = source_w;
    let mut h = source_h;
    while w > 512 || h > 512 {
        w >>= 1;
        h >>= 1;
        level += 1;
    }
    level
}

// ---------------------------------------------------------------------------
// Cache directory helpers
// ---------------------------------------------------------------------------

/// Return the filesystem cache directory for a given source image.
fn cache_dir(source: &Path) -> PathBuf {
    let hash = short_hash(source);
    let config = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".config")
        });
    let base = config.join("y5/wallpaper");
    base.join(format!("{hash}.cache"))
}

/// A short hex hash of the source's canonical path (collisions are harmless —
/// they just merge two wallpapers into one cache dir).
fn short_hash(source: &Path) -> String {
    let canonical = source.canonicalize().unwrap_or_else(|_| source.to_path_buf());
    let mut hasher = DefaultHasher::new();
    canonical.to_string_lossy().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}


