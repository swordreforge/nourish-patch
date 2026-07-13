#[macro_use]
extern crate compositor_developer_debug_instance_record;

use std::collections::HashMap;
use std::path::PathBuf;
use compositor_background_two_draw_tile::{cache_dir, load_or_generate, load_tile_bytes, RectF64, TileIndex};
use compositor_background_two_draw_wallpaper_tex::{create_gles_texture, WallpaperGpuCache, TileBlit};
use smithay::backend::renderer::gles::GlesRenderer;

pub fn build_or_reuse_cache(path: Option<&str>, existing: Option<&mut WallpaperGpuCache>, gles: &mut GlesRenderer) -> Option<WallpaperGpuCache> {
    let source = match path { Some(p) if !p.is_empty() => PathBuf::from(p), _ => return None };
    if let Some(c) = existing { if c.source == source { return None; } }
    match load_or_generate(&source) {
        Ok(index) => {
            let root = cache_dir(&source);
            let (mut textures, mut sizes) = (HashMap::new(), HashMap::new());
            if let Some(lm) = index.levels.first() {
                for row in 0..lm.rows { for col in 0..lm.cols {
                    let key = (0u8, col, row);
                    let (tw, th) = index.tile_dimensions(0, col, row);
                    if let Ok(bytes) = load_tile_bytes(&index, &root, 0, col, row) {
                        if let Ok(tex) = create_gles_texture(gles, &bytes, tw, th) { textures.insert(key, tex); sizes.insert(key, (tw, th)); }
                    }
                }}
            }
            Some(WallpaperGpuCache { index, cache_root: root, textures, sizes, source })
        }
        Err(e) => {
            warn!("wallpaper.base: FAILED to load/generate pyramid for {}: {}", source.display(), e);
            None
        }
    }
}

/// Pre-computed fill mapping: (eff_scale, offset_x, offset_y) in image-pixel coords.
pub struct FillMapping(pub f64, pub f64, pub f64);

pub fn prepare_tiles(cache: &mut WallpaperGpuCache, gles: &mut GlesRenderer, pan: (f32, f32), zoom: f32, output_size: (f32, f32), fm: FillMapping) -> Vec<TileBlit> {
    let idx = &cache.index;
    let (sw, sh) = (output_size.0 as f64, output_size.1 as f64);
    let (iw, ih) = idx.levels.first().map(|l| (l.w as f64, l.h as f64)).unwrap_or((TileIndex::WORLD_W, TileIndex::WORLD_H));
    let (es, ox, oy) = (fm.0, fm.1, fm.2);
    let (vpw, vph) = (sw / zoom as f64, sh / zoom as f64);
    let (vpl, vpt) = (pan.0 as f64 - vpw / 2.0, pan.1 as f64 - vph / 2.0);
    let (il_l, il_t) = ((vpl - ox) / es, (vpt - oy) / es);
    let (il_r, il_b) = (il_l + vpw / es, il_t + vph / es);
    let (vl, vt, vr, vb) = (il_l.max(0.0), il_t.max(0.0), il_r.min(iw), il_b.min(ih));
    if vr <= vl || vb <= vt { return vec![]; }
    // Convert image-pixel visible rect to world coords for covering_tiles().
    let world_vis = RectF64::new(
        vl / iw * TileIndex::WORLD_W,
        vt / ih * TileIndex::WORLD_H,
        (vr - vl) / iw * TileIndex::WORLD_W,
        (vb - vt) / ih * TileIndex::WORLD_H,
    );
    let lod = idx.select_lod(zoom as f64 * es, sw);
    let tiles = idx.covering_tiles(lod, &world_vis);
    if lod as usize >= idx.levels.len() { return vec![]; }
    let mut blits = Vec::with_capacity(tiles.len());
    for (lod, col, row) in &tiles {
        let key = (*lod, *col, *row);
        if !cache.textures.contains_key(&key) {
            if let Ok(bytes) = load_tile_bytes(idx, &cache.cache_root, *lod, *col, *row) {
                let (tw, th) = idx.tile_dimensions(*lod, *col, *row);
                if let Ok(tex) = create_gles_texture(gles, &bytes, tw, th) { cache.textures.insert(key, tex); cache.sizes.insert(key, (tw, th)); }
            }
        }
        if let Some(tex) = cache.textures.get(&key) {
            let (tw, th) = cache.sizes.get(&key).copied().unwrap_or((512, 512));
            let ts = idx.tile_size as f64;
            let (twx, twy) = (ox + *col as f64 * ts * es, oy + *row as f64 * ts * es);
            let (tww, twh) = (tw as f64 * es, th as f64 * es);
            blits.push(TileBlit { texture: tex.clone(), dst: smithay::utils::Rectangle::from_loc_and_size(
                (((twx - vpl) / vpw * sw) as i32, ((twy - vpt) / vph * sh) as i32),
                ((tww / vpw * sw).ceil().max(1.0) as i32, (twh / vph * sh).ceil().max(1.0) as i32),
            ) });
        }
    }
    blits
}
