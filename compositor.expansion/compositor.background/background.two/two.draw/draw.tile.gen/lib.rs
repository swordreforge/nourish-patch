//! Tile pyramid generation: downscale source and slice into LOD tile PNGs.
use std::path::Path;

use compositor_background_two_draw_tile_base::TileError;
use compositor_background_two_draw_tile_core::{LevelMeta, TileIndex};

/// Build the tile pyramid for `source` on disk under `cache_root`.
pub fn generate_pyramid(source: &Path, cache_root: &Path) -> Result<TileIndex, TileError> {
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

        let (clamped_w, clamped_h) = (lod_w, lod_h);

        let cols = clamped_w.div_ceil(tile_size);
        let rows = clamped_h.div_ceil(tile_size);

        levels.push(LevelMeta { level: lod as u8, w: clamped_w, h: clamped_h, cols, rows });

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
                let t = lod_img.crop(x, y, tw, th);
                let tile_path = lod_dir.join(format!("{col:03}_{row:03}.png"));
                t.save(&tile_path)?;
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

/// Compute LOD count so the coarsest level fits in ~1 tile (512×512).
pub fn compute_max_level(source_w: u32, source_h: u32) -> u32 {
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
