//! Wallpaper GPU texture types and GLES texture upload.
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use compositor_background_two_draw_tile::TileIndex;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::gles::{GlesRenderer, GlesTexture};
use smithay::backend::renderer::ImportMem;
use smithay::utils::{Buffer, Physical, Rectangle, Size};

/// Pre-uploaded tile textures for the current wallpaper.
pub struct WallpaperGpuCache {
    pub index: Arc<TileIndex>,
    pub cache_root: PathBuf,
    pub textures: HashMap<(u8, u32, u32), GlesTexture>,
    pub sizes: HashMap<(u8, u32, u32), (u32, u32)>,
    pub source: PathBuf,
}

/// One pre-uploaded tile ready to blit into the render target.
#[derive(Clone)]
pub struct TileBlit {
    pub texture: GlesTexture,
    pub dst: Rectangle<i32, Physical>,
}

/// Upload raw RGBA bytes to a GLES texture.
pub fn create_gles_texture(
    gles: &mut GlesRenderer,
    rgba: &[u8],
    w: u32,
    h: u32,
) -> Result<GlesTexture, ()> {
    ImportMem::import_memory(
        gles,
        rgba,
        Fourcc::Abgr8888,
        Size::from((w as i32, h as i32)),
        false,
    )
    .map_err(|_| ())
}
