//! Wallpaper GPU texture types and lazy tile cache.
//! On-demand vips tile computation — no pre-generation to disk.
use std::collections::HashMap;
use std::path::PathBuf;
use libvips_rs::ops;
use libvips_rs::VipsImage;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::gles::{GlesRenderer, GlesTexture};
use smithay::backend::renderer::ImportMem;
use smithay::utils::{Buffer, Physical, Rectangle, Size};

pub struct WallpaperGpuCache {
    /// The vips source image (lazy — not decoded until tile extraction).
    pub source_img: VipsImage,
    pub source: PathBuf,
    pub source_w: u32,
    pub source_h: u32,
    textures: HashMap<(u32, u32), GlesTexture>,
    pub sizes: HashMap<(u32, u32), (u32, u32)>,
    load_order: Vec<(u32, u32)>,
    max_cache: usize,
}

#[derive(Clone)]
pub struct TileBlit {
    pub texture: GlesTexture,
    pub dst: Rectangle<i32, Physical>,
}

impl WallpaperGpuCache {
    pub fn new(source_img: VipsImage, source: PathBuf) -> Self {
        let (w, h) = (source_img.get_width() as u32, source_img.get_height() as u32);
        Self { source_img, source, source_w: w, source_h: h, textures: HashMap::new(), sizes: HashMap::new(), load_order: Vec::new(), max_cache: 256 }
    }

    /// Ensure a tile at (x, y, tw, th) in source coords is in GPU cache.
    /// Extracts from vips source on-demand, resizes, uploads to GPU.
    pub fn ensure_tile(&mut self, gles: &mut GlesRenderer, x: u32, y: u32, tw: u32, th: u32) -> bool {
        let key = (x, y);
        if self.textures.contains_key(&key) {
            self.load_order.retain(|&k| k != key);
            self.load_order.push(key);
            return true;
        }
        // Extract region from vips source — vips lazily decodes only these pixels.
        let region = match ops::extract_area(&self.source_img, x as i32, y as i32, tw as i32, th as i32) {
            Ok(r) => r,
            Err(_) => return false,
        };
        // Convert to raw RGBA via png roundtrip (forces evaluation of this region only).
        let png = match ops::pngsave_buffer(&region) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let decoded = match image::load_from_memory(&png) {
            Ok(d) => d,
            Err(_) => return false,
        };
        let raw = decoded.into_rgba8().into_raw();
        if let Ok(tex) = create_gles_texture(gles, &raw, tw, th) {
            self.textures.insert(key, tex);
            self.sizes.insert(key, (tw, th));
            self.load_order.push(key);
            self.evict();
            return true;
        }
        false
    }

    pub fn get_texture(&self, x: u32, y: u32) -> Option<&GlesTexture> {
        self.textures.get(&(x, y))
    }

    pub fn get_size(&self, x: u32, y: u32) -> (u32, u32) {
        self.sizes.get(&(x, y)).copied().unwrap_or((512, 512))
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
