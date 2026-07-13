#[macro_use]
extern crate compositor_developer_debug_instance_record;

use std::path::PathBuf;
use std::sync::Arc;
use compositor_background_two_draw_tile::{cache_dir, load_or_generate, RectF64, TileIndex};
use compositor_background_two_draw_wallpaper_tex::{WallpaperGpuCache, TileBlit};
use smithay::backend::renderer::gles::GlesRenderer;

/// Build a lazy cache from a source path (generates pyramid if needed).
pub fn build_or_reuse_cache(path: Option<&str>, existing: Option<&mut WallpaperGpuCache>, gles: &mut GlesRenderer) -> Option<WallpaperGpuCache> {
    let source = match path { Some(p) if !p.is_empty() => PathBuf::from(p), _ => return None };
    if let Some(c) = existing { if c.source == source { return None; } }
    match load_or_generate(&source) {
        Ok(index) => {
            let root = cache_dir(&source);
            Some(WallpaperGpuCache::new(index, root, source))
        }
        Err(e) => {
            warn!("wallpaper.base: FAILED to load/generate pyramid for {}: {}", source.display(), e);
            None
        }
    }
}

/// Build a lazy cache from a pre-generated TileIndex (background thread result).
pub fn build_or_reuse_cache_from_index(
    path: &Option<String>, index: &Arc<TileIndex>, root: &std::path::Path, _gles: &mut GlesRenderer,
) -> Option<WallpaperGpuCache> {
    let source = match path { Some(p) if !p.is_empty() => PathBuf::from(p), _ => return None };
    Some(WallpaperGpuCache::new(index.clone(), root.to_path_buf(), source))
}

/// Pre-computed fill mapping: (eff_scale, offset_x, offset_y) in image-pixel coords.
pub struct FillMapping(pub f64, pub f64, pub f64);

pub fn prepare_tiles(cache: &mut WallpaperGpuCache, gles: &mut GlesRenderer, pan: (f32, f32), zoom: f32, output_size: (f32, f32), fm: FillMapping) -> Vec<TileBlit> {
    let (sw, sh) = (output_size.0 as f64, output_size.1 as f64);
    let (es, ox, oy) = (fm.0, fm.1, fm.2);
    let (vpw, vph) = (sw / zoom as f64, sh / zoom as f64);
    let (vpl, vpt) = (pan.0 as f64 - vpw / 2.0, pan.1 as f64 - vph / 2.0);

    // Extract index metadata before mutable borrow of cache.
    let (iw, ih, tile_size, lod_levels) = {
        let idx = &cache.index;
        let lm = idx.levels.first().map(|l| (l.w as f64, l.h as f64)).unwrap_or((TileIndex::WORLD_W, TileIndex::WORLD_H));
        (lm.0, lm.1, idx.tile_size, idx.levels.len())
    };

    let (il_l, il_t) = ((vpl - ox) / es, (vpt - oy) / es);
    let (il_r, il_b) = (il_l + vpw / es, il_t + vph / es);
    let (vl, vt, vr, vb) = (il_l.max(0.0), il_t.max(0.0), il_r.min(iw), il_b.min(ih));
    if vr <= vl || vb <= vt { return vec![]; }
    let world_vis = RectF64::new(
        vl / iw * TileIndex::WORLD_W,
        vt / ih * TileIndex::WORLD_H,
        (vr - vl) / iw * TileIndex::WORLD_W,
        (vb - vt) / ih * TileIndex::WORLD_H,
    );

    // Select LOD using the index (immutable borrow is brief).
    let lod = {
        let idx = &cache.index;
        idx.select_lod(zoom as f64 * es, sw)
    };
    let tiles = {
        let idx = &cache.index;
        idx.covering_tiles(lod, &world_vis)
    };
    if lod as usize >= lod_levels { return vec![]; }

    let mut blits = Vec::with_capacity(tiles.len());
    // Pass 1: ensure all visible tiles are loaded (mutable borrow).
    for (lod, col, row) in &tiles {
        cache.ensure_tile(gles, *lod, *col, *row);
    }
    // Pass 2: build blits from cached textures (immutable borrow).
    for (lod, col, row) in &tiles {
        if let Some(tex) = cache.get_texture(*lod, *col, *row) {
            let (tw, th) = cache.get_size(*lod, *col, *row);
            let ts = tile_size as f64;
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
