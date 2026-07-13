//! Wallpaper tile computation — vips on-demand, no disk I/O.
#[macro_use]
extern crate compositor_developer_debug_instance_record;
use std::path::PathBuf;
use compositor_background_two_draw_tile::TileIndex;
use compositor_background_two_draw_wallpaper_tex::{WallpaperGpuCache, TileBlit};
use smithay::backend::renderer::gles::GlesRenderer;

/// Pre-computed fill mapping: (eff_scale, offset_x, offset_y) in image-pixel coords.
pub struct FillMapping(pub f64, pub f64, pub f64);

/// Open source image with vips (lazy — no decode yet).
pub fn open_wallpaper(path: &str) -> Option<WallpaperGpuCache> {
    let img = libvips_rs::VipsImage::new_from_file(path).ok()?;
    info!("wallpaper: opened {}x{}", img.get_width(), img.get_height());
    Some(WallpaperGpuCache::new(img, PathBuf::from(path)))
}

pub fn prepare_tiles(cache: &mut WallpaperGpuCache, gles: &mut GlesRenderer, pan: (f32, f32), zoom: f32, output_size: (f32, f32), fm: FillMapping) -> Vec<TileBlit> {
    let (sw, sh) = (output_size.0 as f64, output_size.1 as f64);
    let (iw, ih) = (cache.source_w as f64, cache.source_h as f64);
    let (es, ox, oy) = (fm.0, fm.1, fm.2);
    let (vpw, vph) = (sw / zoom as f64, sh / zoom as f64);
    let (vpl, vpt) = (pan.0 as f64 - vpw / 2.0, pan.1 as f64 - vph / 2.0);
    // Viewport in source-pixel coords.
    let (il_l, il_t) = ((vpl - ox) / es, (vpt - oy) / es);
    let (il_r, il_b) = (il_l + vpw / es, il_t + vph / es);
    let (vl, vt, vr, vb) = (il_l.max(0.0), il_t.max(0.0), il_r.min(iw), il_b.min(ih));
    if vr <= vl || vb <= vt { return vec![]; }

    // Compute which 512×512 source tiles are visible.
    let tile_size = 512.0;
    let col_start = (vl / tile_size).floor() as u32;
    let col_end = (vr / tile_size).ceil() as u32;
    let row_start = (vt / tile_size).floor() as u32;
    let row_end = (vb / tile_size).ceil() as u32;

    // Ensure all visible tiles are loaded into GPU.
    for row in row_start..row_end {
        for col in col_start..col_end {
            let (x, y) = (col * 512, row * 512);
            let tw = 512u32.min(cache.source_w.saturating_sub(x));
            let th = 512u32.min(cache.source_h.saturating_sub(y));
            if tw > 0 && th > 0 {
                cache.ensure_tile(gles, x, y, tw, th);
            }
        }
    }

    // Build blits from cached textures.
    let mut blits = Vec::new();
    for row in row_start..row_end {
        for col in col_start..col_end {
            let (x, y) = (col * 512, row * 512);
            if let Some(tex) = cache.get_texture(x, y) {
                let (tw, th) = cache.get_size(x, y);
                // Map tile from source coords → screen coords.
                let screen_x = (ox + x as f64 * es - vpl) / vpw * sw;
                let screen_y = (oy + y as f64 * es - vpt) / vph * sh;
                let screen_w = (tw as f64 * es / vpw * sw).ceil().max(1.0) as i32;
                let screen_h = (th as f64 * es / vph * sh).ceil().max(1.0) as i32;
                blits.push(TileBlit { texture: tex.clone(), dst: smithay::utils::Rectangle::from_loc_and_size(
                    (screen_x as i32, screen_y as i32), (screen_w, screen_h),
                ) });
            }
        }
    }
    blits
}
