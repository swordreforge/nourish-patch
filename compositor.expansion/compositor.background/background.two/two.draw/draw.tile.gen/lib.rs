//! Tile pyramid generation: downscale source and slice into LOD tile raw RGBA.
//! Uses libvips for streaming decode — never loads the full image into RAM.
//! Each tile: extract source region → resize to tile size (~1MB decode).

#[macro_use]
extern crate compositor_developer_debug_instance_record;

use std::path::Path;

use compositor_background_two_draw_tile_base::TileError;
use compositor_background_two_draw_tile_core::{LevelMeta, TileIndex};
use libvips_rs::ops;
use libvips_rs::{VipsApp, VipsImage};

unsafe extern "C" { fn vips_image_write_to_memory(image: *const std::ffi::c_void, size: *mut usize) -> *mut u8; }

/// Get raw pixel bytes from a vips image (forces decode of this region only).
fn vips_to_raw(img: &VipsImage) -> Result<Vec<u8>, TileError> {
    let mut size: usize = 0;
    let ptr = unsafe { vips_image_write_to_memory(img as *const VipsImage as _, &mut size) };
    if ptr.is_null() || size == 0 {
        return Err(TileError::Io(std::io::Error::new(std::io::ErrorKind::Other, "vips_image_write_to_memory failed")));
    }
    let bytes = unsafe { std::slice::from_raw_parts(ptr, size) }.to_vec();
    unsafe { libc::free(ptr as *mut libc::c_void); }
    Ok(bytes)
}

fn vips_err(ctx: &str, e: impl std::fmt::Display) -> TileError {
    TileError::Io(std::io::Error::new(std::io::ErrorKind::Other, format!("vips {ctx}: {e}")))
}

/// Build the tile pyramid for `source` on disk under `cache_root`.
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
        let (sx, sy) = (source_w as f64 / lod_w as f64, source_h as f64 / lod_h as f64);

        let lod_dir = cache_root.join(format!("L{lod}"));
        std::fs::create_dir_all(&lod_dir)?;

        for row in 0..rows {
            for col in 0..cols {
                let (x, y) = (col * tile_size, row * tile_size);
                let (tw, th) = (tile_size.min(lod_w.saturating_sub(x)), tile_size.min(lod_h.saturating_sub(y)));
                if tw == 0 || th == 0 { continue; }
                // Map tile region to source coords.
                let (src_x, src_y) = ((x as f64 * sx) as i32, (y as f64 * sy) as i32);
                let (src_w, src_h) = ((tw as f64 * sx).ceil() as i32, (th as f64 * sy).ceil() as i32);
                // Extract source region + resize to tile size — vips decodes only these pixels.
                let region = ops::extract_area(&src, src_x, src_y, src_w, src_h).map_err(|e| vips_err("extract", e))?;
                let scale = (tw as f64 / src_w as f64).max(th as f64 / src_h as f64);
                let tile = ops::resize(&region, scale).map_err(|e| vips_err("resize", e))?;
                let raw = vips_to_raw(&tile)?;
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
