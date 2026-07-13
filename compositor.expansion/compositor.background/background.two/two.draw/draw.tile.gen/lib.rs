//! Tile pyramid generation: downscale source and slice into LOD tile raw RGBA.
//! Uses libvips for streaming decode — never loads the full image into RAM.

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

/// Build the tile pyramid for `source` on disk under `cache_root`.
/// Forces each LOD level into memory once, then extracts tiles from the
/// in-memory copy (no per-tile PNG round-trip).
pub fn generate_pyramid(source: &Path, cache_root: &Path) -> Result<TileIndex, TileError> {
    let _app = VipsApp::new("y5_tilegen", false).map_err(|e| vips_err("init", e))?;
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

        // Downscale source to LOD size — vips streams this.
        let scale = (lod_w as f64 / source_w as f64).max(lod_h as f64 / source_h as f64);
        let lod_img = ops::resize(&src, scale).map_err(|e| vips_err("resize", e))?;
        // Force into contiguous memory — one allocation for the whole LOD.
        let lod_mem = VipsImage::image_copy_memory(lod_img.clone()).map_err(|e| vips_err("copy_memory", e))?;
        let lod_stride = lod_mem.get_width() * lod_mem.get_bands();
        // Extract pixels as RGBA via png roundtrip (one per LOD, not per tile).
        let lod_png = ops::pngsave_buffer(&lod_mem).map_err(|e| vips_err("pngsave", e))?;
        let lod_rgba = image::load_from_memory(&lod_png)
            .map_err(|e| vips_err("png decode", e))?
            .into_rgba8();
        let lod_bytes = lod_rgba.as_raw();
        drop(lod_png); drop(lod_mem); drop(lod_img); // free vips memory

        let lod_dir = cache_root.join(format!("L{lod}"));
        std::fs::create_dir_all(&lod_dir)?;

        for row in 0..rows {
            for col in 0..cols {
                let (x, y) = (col * tile_size, row * tile_size);
                let (tw, th) = (tile_size.min(lod_w.saturating_sub(x)), tile_size.min(lod_h.saturating_sub(y)));
                if tw == 0 || th == 0 { continue; }
                // Extract tile from the RGBA buffer — pure pointer arithmetic, no I/O.
                let mut tile_raw = Vec::with_capacity((tw * th * 4) as usize);
                for ty in 0..th {
                    let src_offset = ((y + ty) * lod_w + x) as usize * 4;
                    let src_end = src_offset + tw as usize * 4;
                    tile_raw.extend_from_slice(&lod_bytes[src_offset..src_end]);
                }
                std::fs::write(lod_dir.join(format!("{col:03}_{row:03}.raw")), &tile_raw)?;
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
