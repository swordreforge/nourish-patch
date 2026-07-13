//! Tile pyramid generation: downscale source and slice into LOD tile raw RGBA.

#[macro_use]
extern crate compositor_developer_debug_instance_record;

use std::path::Path;

use compositor_background_two_draw_tile_base::TileError;
use compositor_background_two_draw_tile_core::{LevelMeta, TileIndex};

/// Build the tile pyramid for `source` on disk under `cache_root`.
pub fn generate_pyramid(source: &Path, cache_root: &Path) -> Result<TileIndex, TileError> {
    info!("tile.gen: decoding image {}", source.display());
    let img = image::open(source)?;
    let (source_w, source_h) = (img.width(), img.height());
    info!("tile.gen: image decoded: {}x{}", source_w, source_h);
    if source_w < 512 || source_h < 512 { return Err(TileError::TooSmall { min: 512 }); }

    let tile_size = 512u32;
    let mut levels = Vec::new();
    let max_level = compute_max_level(source_w, source_h);
    std::fs::create_dir_all(cache_root)?;

    for lod in 0..=max_level {
        let lod_w = (source_w as f64 / 2u32.pow(lod) as f64).ceil() as u32;
        let lod_h = (source_h as f64 / 2u32.pow(lod) as f64).ceil() as u32;
        let cols = lod_w.div_ceil(tile_size);
        let rows = lod_h.div_ceil(tile_size);
        levels.push(LevelMeta { level: lod as u8, w: lod_w, h: lod_h, cols, rows });
        info!("tile.gen: LOD {}: {}x{} ({}x{} tiles)", lod, lod_w, lod_h, cols, rows);

        let lod_rgba = img.resize_exact(lod_w, lod_h, image::imageops::FilterType::Lanczos3).into_rgba8();
        let lod_bytes = lod_rgba.as_raw();

        let lod_dir = cache_root.join(format!("L{lod}"));
        std::fs::create_dir_all(&lod_dir)?;

        for row in 0..rows {
            for col in 0..cols {
                let (x, y) = (col * tile_size, row * tile_size);
                let (tw, th) = (tile_size.min(lod_w.saturating_sub(x)), tile_size.min(lod_h.saturating_sub(y)));
                if tw == 0 || th == 0 { continue; }
                // Extract tile from RGBA buffer — pure pointer arithmetic.
                let mut raw = Vec::with_capacity((tw * th * 4) as usize);
                for ty in 0..th {
                    let off = ((y + ty) * lod_w + x) as usize * 4;
                    raw.extend_from_slice(&lod_bytes[off..off + tw as usize * 4]);
                }
                std::fs::write(lod_dir.join(format!("{col:03}_{row:03}.raw")), &raw)?;
            }
        }
    }

    let index = TileIndex { source: source.canonicalize()?, source_w, source_h, tile_size, levels };
    std::fs::write(cache_root.join("index.json"), serde_json::to_string_pretty(&index)?)?;
    info!("tile.gen: pyramid saved to {}", cache_root.display());
    Ok(index)
}

pub fn compute_max_level(source_w: u32, source_h: u32) -> u32 {
    let mut level = 0u32;
    let (mut w, mut h) = (source_w, source_h);
    while w > 512 || h > 512 { w >>= 1; h >>= 1; level += 1; }
    level
}
