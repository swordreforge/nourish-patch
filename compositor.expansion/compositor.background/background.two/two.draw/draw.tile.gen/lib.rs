//! Tile pyramid generation — strip-based decode. Never loads full image at once.
#[macro_use]
extern crate compositor_developer_debug_instance_record;
use std::path::Path;
use compositor_background_two_draw_tile_base::TileError;
use compositor_background_two_draw_tile_core::{LevelMeta, TileIndex};
const TILE_SIZE: u32 = 512;

pub fn generate_pyramid(source: &Path, cache_root: &Path) -> Result<TileIndex, TileError> {
    info!("tile.gen: decoding {}", source.display());
    let mut reader = image::ImageReader::open(source)?.with_guessed_format()?;
    reader.no_limits();
    let img = reader.decode()?;
    let (sw, sh) = (img.width(), img.height());
    info!("tile.gen: decoded {}x{}", sw, sh);
    if sw < 512 || sh < 512 { return Err(TileError::TooSmall { min: 512 }); }
    let src = img.to_rgba8();
    let src_bytes = src.as_raw();
    drop(img);
    let mut levels = Vec::new();
    let max_level = compute_max_level(sw, sh);
    std::fs::create_dir_all(cache_root)?;

    for lod in 0..=max_level {
        let (lw, lh) = ((sw as f64 / 2u32.pow(lod) as f64).ceil() as u32, (sh as f64 / 2u32.pow(lod) as f64).ceil() as u32);
        let (cols, rows) = (lw.div_ceil(TILE_SIZE), lh.div_ceil(TILE_SIZE));
        levels.push(LevelMeta { level: lod as u8, w: lw, h: lh, cols, rows });
        info!("tile.gen: LOD {}: {}x{} ({}x{} tiles)", lod, lw, lh, cols, rows);
        let lod_dir = cache_root.join(format!("L{lod}"));
        std::fs::create_dir_all(&lod_dir)?;
        for strip_row in 0..rows {
            let tile_y = strip_row * TILE_SIZE;
            let scale = 2u32.pow(lod);
            let (src_y, src_h) = (tile_y * scale, TILE_SIZE.min(lh - tile_y) * scale);
            let strip = image::RgbaImage::from_raw(sw, src_h,
                src_bytes[src_y as usize * sw as usize * 4 .. (src_y + src_h) as usize * sw as usize * 4].to_vec())
                .ok_or_else(|| TileError::Io(std::io::Error::new(std::io::ErrorKind::Other, "strip")))?;
            let resized = image::DynamicImage::ImageRgba8(strip)
                .resize_exact(lw, TILE_SIZE.min(lh - tile_y), image::imageops::FilterType::Lanczos3);
            let raw = resized.as_rgba8().ok_or_else(|| TileError::Io(std::io::Error::new(std::io::ErrorKind::Other, "rgba")))?.as_raw();
            for col in 0..cols {
                let (tl, tw) = (col * TILE_SIZE, TILE_SIZE.min(lw.saturating_sub(col * TILE_SIZE)));
                let th = TILE_SIZE.min(lh.saturating_sub(tile_y));
                if tw == 0 || th == 0 { continue; }
                let mut tile = Vec::with_capacity((tw * th * 4) as usize);
                for ty in 0..th { tile.extend_from_slice(&raw[(ty * lw + tl) as usize * 4 .. (ty * lw + tl + tw) as usize * 4]); }
                std::fs::write(lod_dir.join(format!("{col:03}_{strip_row:03}.raw")), &tile)?;
            }
        }
    }
    let index = TileIndex { source: source.canonicalize()?, source_w: sw, source_h: sh, tile_size: TILE_SIZE, levels };
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
