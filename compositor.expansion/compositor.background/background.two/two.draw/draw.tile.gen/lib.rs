//! Tile pyramid generation: downscale source and slice into LOD tile raw RGBA.
//! Each tile: extract source region → resize → encode → decode → write raw.
//! Peak memory: ~3MB per tile (source region + PNG buffer + decoded RGBA).

#[macro_use]
extern crate compositor_developer_debug_instance_record;

use std::path::Path;

use compositor_background_two_draw_tile_base::TileError;
use compositor_background_two_draw_tile_core::{LevelMeta, TileIndex};
use libvips_rs::ops;
use libvips_rs::{VipsApp, VipsImage};

fn vips_err(ctx: &str, e: impl std::fmt::Display) -> TileError {
    TileError::Io(std::io::Error::new(std::io::ErrorKind::Other, format!("vips {ctx}: {e}")))
}

/// Extract raw RGBA bytes from a vips image.
fn vips_to_raw(img: &VipsImage) -> Result<Vec<u8>, TileError> {
    let png = ops::pngsave_buffer(img).map_err(|e| vips_err("pngsave", e))?;
    let decoded = image::load_from_memory(&png).map_err(|e| vips_err("png decode", e))?;
    Ok(decoded.into_rgba8().into_raw())
}

/// Build the tile pyramid for `source` on disk under `cache_root`.
/// Each tile is extracted+resized from the source individually — vips only
/// decodes the PNG pixels needed for each 512x512 tile (~1MB), never
/// materializing the full image or any LOD level.
pub fn generate_pyramid(source: &Path, cache_root: &Path) -> Result<TileIndex, TileError> {
    let app = VipsApp::new("y5_tilegen", false).map_err(|e| vips_err("init", e))?;
    // Disable vips internal cache — we manage tiles ourselves.
    // Without this, vips caches decompressed scanlines and OOMs on large images.
    app.cache_set_max(0);
    app.cache_set_max_mem(0);
    info!("tile.gen: opening image via vips {}", source.display());
    let src = VipsImage::new_from_file(source.to_str().unwrap_or_default())
        .map_err(|e| vips_err("open", e))?;
    let (source_w, source_h) = (src.get_width() as u32, src.get_height() as u32);
    info!("tile.gen: image header: {}x{}", source_w, source_h);
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
        let (sx, sy) = (source_w as f64 / lod_w as f64, source_h as f64 / lod_h as f64);

        let lod_dir = cache_root.join(format!("L{lod}"));
        std::fs::create_dir_all(&lod_dir)?;

        for row in 0..rows {
            for col in 0..cols {
                let (x, y) = (col * tile_size, row * tile_size);
                let (tw, th) = (tile_size.min(lod_w.saturating_sub(x)), tile_size.min(lod_h.saturating_sub(y)));
                if tw == 0 || th == 0 { continue; }
                let (src_x, src_y) = ((x as f64 * sx) as i32, (y as f64 * sy) as i32);
                let (src_w, src_h) = ((tw as f64 * sx).ceil() as i32, (th as f64 * sy).ceil() as i32);
                // Extract source region + resize to tile size — ~1MB decode.
                let region = ops::extract_area(&src, src_x, src_y, src_w, src_h).map_err(|e| vips_err("extract", e))?;
                let scale = (tw as f64 / src_w as f64).max(th as f64 / src_h as f64);
                let tile = ops::resize(&region, scale).map_err(|e| vips_err("resize", e))?;
                let raw = vips_to_raw(&tile)?;
                // tile + region + png drop here — memory freed before next tile.
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
