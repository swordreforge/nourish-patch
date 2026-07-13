//! Tile pyramid disk I/O and cache directory helpers.

#[macro_use]
extern crate compositor_developer_debug_instance_record;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use compositor_background_two_draw_tile_base::TileError;
pub use compositor_background_two_draw_tile_core::{LevelMeta, TileIndex};
use compositor_background_two_draw_tile_gen::generate_pyramid;

/// Load an existing tile pyramid from disk, or build one from the source image.
pub fn load_or_generate(source: &Path) -> Result<Arc<TileIndex>, TileError> {
    let root = cache_dir(source);
    let index_path = root.join("index.json");
    if index_path.exists() {
        info!("tile.io: loading existing pyramid from {}", index_path.display());
        let raw = std::fs::read_to_string(&index_path)?;
        let index: TileIndex = serde_json::from_str(&raw)?;
        return Ok(Arc::new(index));
    }
    warn!("tile.io: generating pyramid for {} (may take a while for large images)", source.display());
    let index = generate_pyramid(source, &root)?;
    info!("tile.io: pyramid generated: {} levels, source {}x{}", index.levels.len(), index.source_w, index.source_h);
    Ok(Arc::new(index))
}

/// Load the raw RGBA bytes of a single tile from disk.
pub fn load_tile_bytes(
    index: &TileIndex,
    cache_root: &Path,
    lod: u8,
    col: u32,
    row: u32,
) -> Result<Vec<u8>, TileError> {
    let lm = index.level(lod)?;
    if col >= lm.cols || row >= lm.rows {
        return Err(TileError::InvalidLod(lod));
    }
    let tile_path = cache_root
        .join(format!("L{lod}"))
        .join(format!("{col:03}_{row:03}.png"));
    let img = image::open(&tile_path)?;
    Ok(img.into_rgba8().into_raw())
}

/// Return the filesystem cache directory for a given source image.
pub fn cache_dir(source: &Path) -> PathBuf {
    let hash = short_hash(source);
    let config = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".config")
        });
    config.join("y5/wallpaper").join(format!("{hash}.cache"))
}

fn short_hash(source: &Path) -> String {
    let canonical = source.canonicalize().unwrap_or_else(|_| source.to_path_buf());
    let mut hasher = DefaultHasher::new();
    canonical.to_string_lossy().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
