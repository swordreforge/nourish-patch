use std::collections::HashMap;
use std::path::PathBuf;

use compositor_background_two_draw_tile::{cache_dir, load_or_generate, load_tile_bytes, RectF64, TileIndex};
use compositor_background_two_draw_wallpaper_tex::{create_gles_texture, WallpaperGpuCache, TileBlit};
use smithay::backend::renderer::gles::GlesRenderer;
pub fn build_or_reuse_cache(
    path: Option<&str>,
    existing: Option<&mut WallpaperGpuCache>,
    gles: &mut GlesRenderer,
) -> Option<WallpaperGpuCache> {
    let source = match path {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => return None,
    };
    if let Some(cache) = existing {
        if cache.source == source {
            return None;
        }
    }
    let index = load_or_generate(&source).ok()?;
    let root = cache_dir(&source);
    let mut textures = HashMap::new();
    let mut sizes = HashMap::new();
    if let Some(lm) = index.levels.first() {
        for row in 0..lm.rows {
            for col in 0..lm.cols {
                let key = (0u8, col, row);
                let (tw, th) = index.tile_dimensions(0, col, row);
                if let Ok(bytes) = load_tile_bytes(&index, &root, 0, col, row) {
                    if let Ok(tex) = create_gles_texture(gles, &bytes, tw, th) {
                        textures.insert(key, tex);
                        sizes.insert(key, (tw, th));
                    }
                }
            }
        }
    }
    Some(WallpaperGpuCache { index, cache_root: root, textures, sizes, source })
}

pub fn prepare_tiles(
    cache: &mut WallpaperGpuCache,
    gles: &mut GlesRenderer,
    pan: (f32, f32),
    zoom: f32,
    output_size: (f32, f32),
) -> Vec<TileBlit> {
    let index = &cache.index;
    let screen_w = output_size.0 as f64;
    let screen_h = output_size.1 as f64;
    let vp_w = screen_w / zoom as f64;
    let vp_h = screen_h / zoom as f64;
    let vp_left = pan.0 as f64 - vp_w / 2.0;
    let vp_top = pan.1 as f64 - vp_h / 2.0;
    let vl = vp_left.max(0.0);
    let vt = vp_top.max(0.0);
    let vr = (vp_left + vp_w).min(TileIndex::WORLD_W);
    let vb = (vp_top + vp_h).min(TileIndex::WORLD_H);
    if vr <= vl || vb <= vt {
        return vec![];
    }
    let visible = RectF64::new(vl, vt, vr - vl, vb - vt);
    let lod = index.select_lod(zoom as f64, screen_w);
    let tiles = index.covering_tiles(lod, &visible);
    if lod as usize >= index.levels.len() {
        return vec![];
    }
    let lm = &index.levels[lod as usize];
    let mut blits = Vec::with_capacity(tiles.len());
    for (lod, col, row) in &tiles {
        let key = (*lod, *col, *row);
        if !cache.textures.contains_key(&key) {
            if let Ok(bytes) = load_tile_bytes(index, &cache.cache_root, *lod, *col, *row) {
                let (tw, th) = index.tile_dimensions(*lod, *col, *row);
                if let Ok(tex) = create_gles_texture(gles, &bytes, tw, th) {
                    cache.textures.insert(key, tex);
                    cache.sizes.insert(key, (tw, th));
                }
            }
        }
        if let Some(tex) = cache.textures.get(&key) {
            let (tw, th) = cache.sizes.get(&key).copied().unwrap_or((512, 512));
            let ts = index.tile_size as f64;
            let tile_img_x = *col as f64 * ts;
            let tile_img_y = *row as f64 * ts;
            let tile_world_x = tile_img_x / lm.w as f64 * TileIndex::WORLD_W;
            let tile_world_y = tile_img_y / lm.h as f64 * TileIndex::WORLD_H;
            let tile_world_w = tw as f64 / lm.w as f64 * TileIndex::WORLD_W;
            let tile_world_h = th as f64 / lm.h as f64 * TileIndex::WORLD_H;
            let sx = ((tile_world_x - vp_left) / vp_w * screen_w) as i32;
            let sy = ((tile_world_y - vp_top) / vp_h * screen_h) as i32;
            let sw = (tile_world_w / vp_w * screen_w).ceil() as i32;
            let sh = (tile_world_h / vp_h * screen_h).ceil() as i32;
            let dst = smithay::utils::Rectangle::from_loc_and_size((sx, sy), (sw.max(1), sh.max(1)));
            blits.push(TileBlit { texture: tex.clone(), dst });
        }
    }
    blits
}
