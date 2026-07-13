//! Wallpaper GPU texture types, GLES texture upload, and lazy tile cache.
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use compositor_background_two_draw_tile::TileIndex;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::gles::{GlesRenderer, GlesTexture};
use smithay::backend::renderer::ImportMem;
use smithay::utils::{Buffer, Physical, Rectangle, Size};

/// Lazy tile cache: tiles loaded on-demand, LRU eviction bounds GPU memory.
pub struct WallpaperGpuCache {
    pub index: Arc<TileIndex>,
    pub cache_root: PathBuf,
    pub source: PathBuf,
    textures: HashMap<(u8, u32, u32), GlesTexture>,
    pub sizes: HashMap<(u8, u32, u32), (u32, u32)>,
    load_order: Vec<(u8, u32, u32)>,
    max_cache: usize,
}

#[derive(Clone)]
pub struct TileBlit {
    pub texture: GlesTexture,
    pub dst: Rectangle<i32, Physical>,
}

impl WallpaperGpuCache {
    pub fn new(index: Arc<TileIndex>, cache_root: PathBuf, source: PathBuf) -> Self {
        Self { index, cache_root, source, textures: HashMap::new(), sizes: HashMap::new(), load_order: Vec::new(), max_cache: 256 }
    }

    pub fn ensure_tile(&mut self, gles: &mut GlesRenderer, lod: u8, col: u32, row: u32) -> bool {
        let key = (lod, col, row);
        if self.textures.contains_key(&key) {
            self.load_order.retain(|&k| k != key);
            self.load_order.push(key);
            return true;
        }
        if let Ok(bytes) = compositor_background_two_draw_tile::load_tile_bytes(&self.index, &self.cache_root, lod, col, row) {
            let (tw, th) = self.index.tile_dimensions(lod, col, row);
            if let Ok(tex) = create_gles_texture(gles, &bytes, tw, th) {
                self.textures.insert(key, tex); self.sizes.insert(key, (tw, th));
                self.load_order.push(key); self.evict();
                return true;
            }
        }
        false
    }

    pub fn get_texture(&self, lod: u8, col: u32, row: u32) -> Option<&GlesTexture> {
        self.textures.get(&(lod, col, row))
    }

    pub fn get_size(&self, lod: u8, col: u32, row: u32) -> (u32, u32) {
        self.sizes.get(&(lod, col, row)).copied().unwrap_or((512, 512))
    }

    fn evict(&mut self) {
        while self.load_order.len() > self.max_cache {
            if let Some(oldest) = self.load_order.first().copied() {
                self.load_order.remove(0); self.textures.remove(&oldest); self.sizes.remove(&oldest);
            } else { break; }
        }
    }
}

pub fn create_gles_texture(gles: &mut GlesRenderer, rgba: &[u8], w: u32, h: u32) -> Result<GlesTexture, ()> {
    ImportMem::import_memory(gles, rgba, Fourcc::Abgr8888, Size::from((w as i32, h as i32)), false).map_err(|_| ())
}
