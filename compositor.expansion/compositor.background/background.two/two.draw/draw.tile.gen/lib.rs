//! Tile generation — now handled by vips on-demand in draw.wallpaper.base.
//! This crate is kept for backward compatibility but is no longer the primary path.
use std::path::Path;
use compositor_background_two_draw_tile_base::TileError;
use compositor_background_two_draw_tile_core::{LevelMeta, TileIndex};

pub fn generate_pyramid(_source: &Path, _cache_root: &Path) -> Result<TileIndex, TileError> {
    // No-op: tiles are now computed on-demand by vips in WallpaperGpuCache.
    Err(TileError::Io(std::io::Error::new(std::io::ErrorKind::Other, "use vips on-demand")))
}

pub fn compute_max_level(source_w: u32, source_h: u32) -> u32 {
    let mut level = 0u32;
    let (mut w, mut h) = (source_w, source_h);
    while w > 512 || h > 512 { w >>= 1; h >>= 1; level += 1; }
    level
}
