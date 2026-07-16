use compositor_background_two_draw_element::element::{ParallaxBackground, WallpaperTile};
use compositor_background_two_state_base::state::Two;
use compositor_support_system_buffer_token_base::y5_buffer;
use compositor_support_system_trait_system_base::base::{BufferCx, System, SystemCx, WorldBuilder};
use compositor_support_system_world_frame_base::base::{self as layer, FramePlan, FrameTick};
use image::GenericImageView;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::gles::GlesTexture;
use smithay::backend::renderer::{ImportDma, ImportMem, Texture};
use smithay::backend::allocator::Fourcc;
use smithay::utils::{Buffer as BufferCoord, Size};
use std::any::Any;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::Path;

use lru::LruCache;

use compositor_background_two_storage_base::base::{BG_TWO, BG_TWO_MUT};

enum TwoCmd {
    SetInstance(ParallaxBackground),
    SetWallpaperTiles(Vec<WallpaperTile>),
    Tick,
    Pan(f32, f32),
    Zoom(f32),
    Resize(f32, f32),
}
y5_buffer!(TWO_BUF: TwoCmd);

/// Per-zoom-level grid metadata, detected from the tile directory.
#[derive(Debug, Clone, Copy)]
struct LevelInfo {
    cols: u32,
    rows: u32,
    pixel_w: f32,
    pixel_h: f32,
}

/// Detected info from a vips dzsave --layout google tile pyramid.
struct WallpaperInfo {
    tile_w: f32,
    tile_h: f32,
    max_zoom: u32,
    levels: Vec<LevelInfo>,
}

impl WallpaperInfo {
    /// Actual pixel size of tile (x, y) at the given zoom level.
    fn tile_pixel_size(&self, zoom: u32, x: u32, y: u32) -> (f32, f32) {
        let info = &self.levels[zoom as usize];
        let mut w = self.tile_w;
        let mut h = self.tile_h;
        if x == info.cols - 1 && info.cols > 0 {
            w = info.pixel_w - (info.cols - 1) as f32 * self.tile_w;
        }
        if y == info.rows - 1 && info.rows > 0 {
            h = info.pixel_h - (info.rows - 1) as f32 * self.tile_h;
        }
        (w.max(1.0), h.max(1.0))
    }

    /// World-space size of one tile pixel at the given zoom level.
    fn tile_scale_factor(&self, zoom: u32) -> f32 {
        (1u32 << (self.max_zoom - zoom)) as f32
    }
}

pub struct TwoSystem {
    tile_cache: LruCache<(u32, i32, i32), (GlesTexture, u32, u32)>,
    dmabufs: HashMap<(u32, i32, i32), Dmabuf>,
    last_wallpaper: Option<String>,
    info: Option<WallpaperInfo>,
    last_pan: (f32, f32),
    last_zoom: f32,
    cached_tiles: Vec<WallpaperTile>,
    async_rx: Option<std::sync::mpsc::Receiver<Vec<((u32, i32, i32), Vec<u8>, u32, u32)>>>,
    needs_rebuild: bool,
}

impl Default for TwoSystem {
    fn default() -> Self {
        Self {
            tile_cache: LruCache::new(NonZeroUsize::new(512).unwrap()),
            dmabufs: HashMap::new(),
            last_wallpaper: None,
            info: None,
            last_pan: (0.0, 0.0),
            last_zoom: 1.0,
            cached_tiles: Vec::new(),
            async_rx: None,
            needs_rebuild: false,
        }
    }
}

impl System for TwoSystem {
    fn name(&self) -> &'static str { "background.two" }

    fn register(&mut self, builder: &mut WorldBuilder) {
        builder.storage.insert(&BG_TWO, Two::new());
        builder.receive(&compositor_y5_camera_system_base::base::CAMERA_MOVED, Self::on_camera_moved);
        builder.receive(&compositor_y5_camera_system_base::base::CAMERA_ZOOMED, Self::on_camera_zoomed);
    }

    fn update(&mut self, cx: &mut SystemCx, _tick: &FrameTick) {
        if cx.storage.try_get(&compositor_background_three_system_base::base::BG_THREE)
            .is_some_and(|b| b.example_lock_done) { return; }
        let size = cx.kernel.get(&compositor_orchestration_smithay_data_base::data::SCREEN).size;
        let size = (size.w as f32, size.h as f32);
        let state = cx.storage.get(&BG_TWO);
        let stale = state.instance.as_ref().is_some_and(|i| i.output_size != size);
        let rebuild = state.instance.is_none();
        let override_sel = state.background_shader.clone();
        let params = state.params.clone();
        let invert_pan_x = state.invert_pan_x;
        let invert_pan_y = state.invert_pan_y;
        let srgb = state.srgb;
        if rebuild {
            if let Some(renderer) = cx
                .platform.as_deref_mut()
                .and_then(|p| p.downcast_mut::<compositor_orchestration_draw_platform_base::platform::Platform>())
                .and_then(|p| p.renderer())
            {
                let sel = override_sel.or_else(
                    compositor_developer_stats_registry_base::base::background_shader_default,
                );
                let mut instance = ParallaxBackground::new(renderer, size, sel.as_deref(), &params);
                instance.invert_pan_x = invert_pan_x;
                instance.invert_pan_y = invert_pan_y;
                instance.srgb = srgb;
                cx.write(&TWO_BUF, TwoCmd::SetInstance(instance));
            }
            return;
        }
        if stale {
            cx.write(&TWO_BUF, TwoCmd::Resize(size.0, size.1));
        }
        cx.write(&TWO_BUF, TwoCmd::Tick);
        self.load_wallpaper_tiles(state, cx, size);
    }

    fn draw(&mut self, cx: &mut SystemCx, plan: &mut FramePlan) {
        if cx.storage.try_get(&compositor_background_three_system_base::base::BG_THREE)
            .is_some_and(|b| b.example_lock_done) { return; }
        if let Some(instance) = &cx.storage.get(&BG_TWO).instance {
            plan.push(layer::BACKGROUND, Box::new(instance.clone()));
        }
    }

    fn buffer(&mut self, cx: &mut BufferCx, message: Box<dyn Any>) {
        let two = cx.storage.get_mut(&BG_TWO_MUT);
        match *message.downcast::<TwoCmd>().expect("two buffer type") {
            TwoCmd::SetInstance(instance) => { two.shader_error = instance.shader_error.clone(); two.instance = Some(instance); }
            TwoCmd::SetWallpaperTiles(tiles) => { if let Some(i) = &mut two.instance { i.set_wallpaper_tiles(tiles); } }
            TwoCmd::Tick => { if let Some(i) = &mut two.instance { i.update(); } }
            TwoCmd::Pan(x, y) => { if let Some(i) = &mut two.instance { i.pan = (x, y); } }
            TwoCmd::Zoom(z) => { if let Some(i) = &mut two.instance { i.zoom = z; } }
            TwoCmd::Resize(w, h) => { if let Some(i) = &mut two.instance { i.output_size = (w, h); } }
        }
    }

    fn persist(&self) -> &'static [&'static compositor_support_system_persist_entry_base::base::PersistEntry] {
        compositor_background_two_storage_base::base::BACKGROUND_PERSISTS
    }
}

impl TwoSystem {
    fn on_camera_moved(&mut self, cx: &mut SystemCx, event: &compositor_y5_camera_system_base::base::CameraMoved) {
        cx.write(&TWO_BUF, TwoCmd::Pan(event.x as f32, event.y as f32));
    }
    fn on_camera_zoomed(&mut self, cx: &mut SystemCx, event: &compositor_y5_camera_system_base::base::CameraZoomed) {
        cx.write(&TWO_BUF, TwoCmd::Zoom(event.zoom as f32));
    }

    fn load_wallpaper_tiles(&mut self, state: &Two, cx: &mut SystemCx, size: (f32, f32)) {
        let wallpaper_path = match &state.wallpaper_path {
            Some(p) if !p.is_empty() => p.clone(),
            _ => {
                if self.last_wallpaper.is_some() {
                    self.last_wallpaper = None;
                    self.info = None;
                    self.tile_cache.clear();
                    self.cached_tiles.clear();
                    cx.write(&TWO_BUF, TwoCmd::SetWallpaperTiles(Vec::new()));
                }
                return;
            }
        };
        if self.last_wallpaper.as_ref() != Some(&wallpaper_path) {
            self.last_wallpaper = Some(wallpaper_path.clone());
            self.info = self.detect_wallpaper_info(&wallpaper_path);
            self.tile_cache.clear();
            self.cached_tiles.clear();
            self.needs_rebuild = true;
        }
        let renderer = match cx
            .platform.as_deref_mut()
            .and_then(|p| p.downcast_mut::<compositor_orchestration_draw_platform_base::platform::Platform>())
            .and_then(|p| p.renderer())
        {
            Some(r) => r,
            None => return,
        };
        let instance = match &state.instance { Some(i) => i, None => return };
        let camera_changed = (instance.pan.0 - self.last_pan.0).abs() > 0.1
            || (instance.pan.1 - self.last_pan.1).abs() > 0.1
            || (instance.zoom - self.last_zoom).abs() > 0.01;
        if camera_changed || self.cached_tiles.is_empty() || self.needs_rebuild {
            self.last_pan = instance.pan;
            self.last_zoom = instance.zoom;
            self.needs_rebuild = false;
            let use_dmabuf = compositor_developer_environment_config_base::base::get().renderer == "vulkan";
            let new_tiles = self.compute_visible_tiles(
                renderer, &wallpaper_path, instance.pan, instance.zoom,
                instance.output_size, use_dmabuf,
            );
            // Only update if: non-empty AND no async loading in progress.
            // Async loading means some tiles are missing from the list.
            if !new_tiles.is_empty() && self.async_rx.is_none() {
                self.cached_tiles = new_tiles;
            }
        }
        cx.write(&TWO_BUF, TwoCmd::SetWallpaperTiles(self.cached_tiles.clone()));
    }

    fn detect_wallpaper_info(&self, wallpaper_path: &str) -> Option<WallpaperInfo> {
        let tiles_dir = Path::new(wallpaper_path).join("tiles");
        if !tiles_dir.is_dir() { return None; }
        let max_zoom = (0u32..30).filter(|z| tiles_dir.join(z.to_string()).is_dir()).max()?;
        // Detect tile size from first tile at max zoom.
        let zoom_dir = tiles_dir.join(max_zoom.to_string());
        let first_y = std::fs::read_dir(&zoom_dir).ok()?
            .filter_map(|e| e.ok()).find(|e| e.path().is_dir())?.path();
        let first_img = std::fs::read_dir(&first_y).ok()?
            .filter_map(|e| e.ok())
            .find(|e| e.path().extension().is_some_and(|ext| ext == "png" || ext == "jpg" || ext == "webp"))?;
        let img = image::open(first_img.path()).ok()?;
        let (tile_w, tile_h) = img.dimensions();
        let tile_w = tile_w as f32;
        let tile_h = tile_h as f32;
        // Detect per-level info.
        let mut levels = Vec::new();
        for z in 0..=max_zoom {
            let z_dir = tiles_dir.join(z.to_string());
            let mut rows = 0u32;
            let mut max_cols = 0u32;
            for y_dir in std::fs::read_dir(&z_dir).ok()?.filter_map(|e| e.ok()) {
                if !y_dir.path().is_dir() { continue; }
                let y: u32 = y_dir.file_name().to_str()?.parse().ok()?;
                rows = rows.max(y + 1);
                let mut cols = 0u32;
                for f in std::fs::read_dir(y_dir.path()).ok()?.filter_map(|e| e.ok()) {
                    let p = f.path();
                    if p.extension().is_some_and(|ext| ext == "png" || ext == "jpg" || ext == "webp") {
                        let x: u32 = p.file_stem()?.to_str()?.parse().ok()?;
                        cols = cols.max(x + 1);
                    }
                }
                max_cols = max_cols.max(cols);
            }
            if rows == 0 || max_cols == 0 { return None; }
            // Find actual extension
            let actual_ext = if z_dir.join("0").join("0.png").exists() { "png" }
                else if z_dir.join("0").join("0.webp").exists() { "webp" }
                else { "jpg" };
            // Read edge tiles to get actual image dimensions.
            let last_col_w = image::open(z_dir.join("0").join(format!("0.{actual_ext}"))).ok()
                .map(|img| img.width() as f32).unwrap_or(tile_w);
            let last_row_h = image::open(z_dir.join((rows - 1).to_string()).join(format!("0.{actual_ext}"))).ok()
                .map(|img| img.height() as f32).unwrap_or(tile_h);
            let pixel_w = (max_cols - 1) as f32 * tile_w + last_col_w;
            let pixel_h = (rows - 1) as f32 * tile_h + last_row_h;
            levels.push(LevelInfo { cols: max_cols, rows, pixel_w, pixel_h });
        }
        Some(WallpaperInfo { tile_w, tile_h, max_zoom, levels })
    }

    fn compute_visible_tiles(
        &mut self,
        renderer: &mut GlesRenderer,
        wallpaper_path: &str,
        pan: (f32, f32),
        zoom: f32,
        output_size: (f32, f32),
        use_dmabuf: bool,
    ) -> Vec<WallpaperTile> {
        let info = match &self.info { Some(i) => i, None => return Vec::new() };
        let max_z = info.max_zoom;
        let img_w = info.levels[0].pixel_w;
        let img_h = info.levels[0].pixel_h;

        // Select zoom level: tile screen size ≈ tile pixel size.
        let tile_z = ((max_z as f32 + zoom.log2()).round() as u32).clamp(0, max_z);
        let sf = info.tile_scale_factor(tile_z); // world pixels per tile pixel
        let tile_world = info.tile_w * sf; // standard tile world-space width

        // Viewport in image coordinates (image origin at top-left).
        // y5 world origin is image center, so image_x = world_x + img_w/2.
        let img_cx = pan.0 + img_w / 2.0;
        let img_cy = pan.1 + img_h / 2.0;
        let vis_left = img_cx - output_size.0 / (2.0 * zoom);
        let vis_top = img_cy - output_size.1 / (2.0 * zoom);
        let vis_right = img_cx + output_size.0 / (2.0 * zoom);
        let vis_bottom = img_cy + output_size.1 / (2.0 * zoom);

        let level = &info.levels[tile_z as usize];
        let x_start = ((vis_left / tile_world).floor() as i32).max(0) as u32;
        let y_start = ((vis_top / tile_world).floor() as i32).max(0) as u32;
        let x_end = ((vis_right / tile_world).ceil() as u32).min(level.cols);
        let y_end = ((vis_bottom / tile_world).ceil() as u32).min(level.rows);

        if x_start >= x_end || y_start >= y_end { return Vec::new(); }

        let tiles_dir = Path::new(wallpaper_path).join("tiles");
        let mut result = Vec::new();
        let mut missing_tiles: Vec<(u32, u32, u32)> = Vec::new();

        // Phase 1: find tiles in cache, record missing ones.
        for ty in y_start..y_end {
            for tx in x_start..x_end {
                let mut found = None;
                // Search DOWN from tile_z to 0.
                for fallback_z in (0..=tile_z).rev() {
                    let d = tile_z - fallback_z;
                    let px = tx >> d;
                    let py = ty >> d;
                    let key = (fallback_z, py as i32, px as i32);
                    if self.tile_cache.contains(&key)
                        && (!use_dmabuf || self.dmabufs.contains_key(&key))
                    {
                        found = Some((key, fallback_z));
                        break;
                    }
                    // Record ALL missing tiles in the fallback chain.
                    if !missing_tiles.iter().any(|&(z, x, y)| z == fallback_z && x == px && y == py) {
                        missing_tiles.push((fallback_z, px, py));
                    }
                }
                // UP search: check higher zoom levels for child tiles.
                if found.is_none() {
                    'up: for higher_z in (tile_z + 1)..=max_z {
                        let d = higher_z - tile_z;
                        let base_x = tx << d;
                        let base_y = ty << d;
                        for dy in 0..(1u32 << d) {
                            for dx in 0..(1u32 << d) {
                                let key = (higher_z, (base_y + dy) as i32, (base_x + dx) as i32);
                                if self.tile_cache.contains(&key)
                                    && (!use_dmabuf || self.dmabufs.contains_key(&key))
                                {
                                    found = Some((key, higher_z));
                                    break 'up;
                                }
                            }
                        }
                    }
                }

                let (key, found_z) = match found {
                    Some(v) => v,
                    None => continue,
                };
                let (texture, tex_w, tex_h) = match self.tile_cache.get(&key).cloned() {
                    Some(v) => v,
                    None => continue,
                };

                // Position and size in world coordinates.
                let tile_sf = info.tile_scale_factor(found_z);
                let (actual_w, actual_h) = info.tile_pixel_size(found_z, (key.2) as u32, key.1 as u32);
                let world_w = actual_w * tile_sf;
                let world_h = actual_h * tile_sf;

                // Tile center in image coordinates.
                let d = tile_z - found_z;
                let render_tx = if d > 0 { tx >> d } else { tx };
                let render_ty = if d > 0 { ty >> d } else { ty };
                let tile_img_x = render_tx as f32 * info.tile_w * tile_sf;
                let tile_img_y = render_ty as f32 * info.tile_h * tile_sf;

                // Convert to world coordinates (origin at image center).
                let tile_world_x = tile_img_x - img_w / 2.0;
                let tile_world_y = tile_img_y - img_h / 2.0;

                // Convert to screen coordinates.
                let sx = ((tile_world_x - pan.0) * zoom + output_size.0 / 2.0).floor();
                let sy = ((tile_world_y - pan.1) * zoom + output_size.1 / 2.0).floor();
                let sw = (world_w * zoom).ceil();
                let sh = (world_h * zoom).ceil();

                if sx + sw <= 0.0 || sx >= output_size.0 || sy + sh <= 0.0 || sy >= output_size.1 {
                    continue;
                }

                let dmabuf = if use_dmabuf {
                    self.dmabufs.get(&key).cloned()
                } else { None };

                result.push(WallpaperTile {
                    x: sx as i32, y: sy as i32,
                    w: sw as i32, h: sh as i32,
                    tex_w, tex_h, texture, dmabuf,
                });
            }
        }

        // Phase 2: async load missing tiles.
        if let Some(rx) = self.async_rx.take() {
            if let Ok(loaded) = rx.try_recv() {
                for (key, data, w, h) in loaded {
                    if use_dmabuf {
                        if let Some(dm) = Self::upload_dmabuf_tile(renderer, &data, w, h) {
                            if let Some(tex) = renderer.import_dmabuf(&dm, None).ok() {
                                self.tile_cache.put(key, (tex, w, h));
                                self.dmabufs.insert(key, dm);
                            }
                        }
                    } else if let Some(tex) = renderer.import_memory(&data, Fourcc::Abgr8888, Size::from((w as i32, h as i32)), false).ok() {
                        self.tile_cache.put(key, (tex, w, h));
                    }
                }
                self.needs_rebuild = true;
            } else {
                self.async_rx = Some(rx);
            }
        }
        if !missing_tiles.is_empty() && self.async_rx.is_none() {
            let tiles_dir_clone = tiles_dir.clone();
            let missing_clone = missing_tiles.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            self.async_rx = Some(rx);
            std::thread::spawn(move || {
                use rayon::prelude::*;
                let loaded: Vec<_> = missing_clone.par_iter()
                    .filter_map(|&(zoom, x, y)| {
                        let base = tiles_dir_clone.join(zoom.to_string()).join(y.to_string());
                        let (data, w, h) = Self::load_tile_image(&base.join(format!("{x}.png")))
                            .or_else(|| Self::load_tile_image(&base.join(format!("{x}.webp"))))
                            .or_else(|| Self::load_tile_image(&base.join(format!("{x}.jpg"))))?;
                        Some(((zoom, y as i32, x as i32), data, w, h))
                    })
                    .collect();
                let _ = tx.send(loaded);
            });
        }

        // Phase 3: prefetch zoom+1 for center 25%.
        if tile_z < max_z {
            let pz = tile_z + 1;
            let psf = info.tile_scale_factor(pz);
            let ptw = info.tile_w * psf;
            let plevel = &info.levels[pz as usize];
            let margin = 0.25;
            let half_w = output_size.0 / (2.0 * zoom) * margin;
            let half_h = output_size.1 / (2.0 * zoom) * margin;
            let px_start = ((img_cx - half_w) / ptw).floor().max(0.0) as u32;
            let py_start = ((img_cy - half_h) / ptw).floor().max(0.0) as u32;
            let px_end = ((img_cx + half_w) / ptw).ceil() as u32;
            let py_end = ((img_cy + half_h) / ptw).ceil() as u32;
            let mut prefetch_missing = Vec::new();
            for py in py_start..py_end.min(plevel.rows) {
                for px in px_start..px_end.min(plevel.cols) {
                    let key = (pz, py as i32, px as i32);
                    if !self.tile_cache.contains(&key) {
                        prefetch_missing.push((pz, px, py));
                    }
                }
            }
            if !prefetch_missing.is_empty() {
                let loaded = Self::batch_load_tiles(renderer, &tiles_dir, &prefetch_missing, use_dmabuf);
                for (key, tex, dmabuf, tw, th) in loaded {
                    self.tile_cache.put(key, (tex, tw, th));
                    if let Some(dm) = dmabuf { self.dmabufs.insert(key, dm); }
                }
            }
        }

        result
    }

    fn load_tile_image(path: &Path) -> Option<(Vec<u8>, u32, u32)> {
        let img = image::open(path).ok()?.into_rgba8();
        let (w, h) = img.dimensions();
        Some((img.into_raw(), w, h))
    }

    fn upload_dmabuf_tile(renderer: &mut GlesRenderer, data: &[u8], w: u32, h: u32) -> Option<Dmabuf> {
        use compositor_support_bevy_core_alloc_base::allocate_dmabuf;
        use smithay::backend::allocator::Buffer;
        use smithay::backend::allocator::dmabuf::DmabufMappingMode;
        use smithay::backend::renderer::ImportDma;
        let allocated = allocate_dmabuf("/dev/dri/renderD128", w, h).ok()?;
        let mapping = allocated.dmabuf.map_plane(0, DmabufMappingMode::READ | DmabufMappingMode::WRITE).ok()?;
        let ptr = mapping.ptr() as *mut u8;
        let len = mapping.length();
        let copy_len = ((w * h * 4) as usize).min(len);
        for i in (0..copy_len).step_by(4) {
            unsafe {
                let r = *data.as_ptr().add(i);
                let b = *data.as_ptr().add(i + 2);
                *ptr.add(i) = b;
                *ptr.add(i + 1) = *data.as_ptr().add(i + 1);
                *ptr.add(i + 2) = r;
                *ptr.add(i + 3) = *data.as_ptr().add(i + 3);
            }
        }
        drop(mapping);
        let tex = renderer.import_dmabuf(&allocated.dmabuf, None).ok()?;
        if let Some(images) = tex.egl_images() {
            if let Some(&image) = images.first() {
                let display = renderer.egl_context().display();
                let size = Size::from((w as i32, h as i32));
                display.create_dmabuf_from_image(image, size, tex.is_y_inverted()).ok()
            } else { Some(allocated.dmabuf) }
        } else { Some(allocated.dmabuf) }
    }

    fn batch_load_tiles(
        renderer: &mut GlesRenderer, tiles_dir: &Path,
        missing: &[(u32, u32, u32)], use_dmabuf: bool,
    ) -> Vec<((u32, i32, i32), GlesTexture, Option<Dmabuf>, u32, u32)> {
        use rayon::prelude::*;
        use compositor_support_bevy_core_alloc_base::allocate_dmabuf;
        use smithay::backend::allocator::Buffer;
        use smithay::backend::allocator::dmabuf::DmabufMappingMode;
        use smithay::backend::renderer::ImportDma;
        let loaded: Vec<_> = missing.par_iter()
            .filter_map(|&(zoom, x, y)| {
                let base = tiles_dir.join(zoom.to_string()).join(y.to_string());
                let (data, w, h) = Self::load_tile_image(&base.join(format!("{x}.png")))
                    .or_else(|| Self::load_tile_image(&base.join(format!("{x}.webp"))))
                    .or_else(|| Self::load_tile_image(&base.join(format!("{x}.jpg"))))?;
                Some((zoom, y as i32, x as i32, data, w, h))
            })
            .collect();
        if use_dmabuf {
            loaded.into_iter().filter_map(|(zoom, y, x, data, w, h)| {
                let allocated = allocate_dmabuf("/dev/dri/renderD128", w, h).ok()?;
                let mapping = allocated.dmabuf.map_plane(0, DmabufMappingMode::READ | DmabufMappingMode::WRITE).ok()?;
                let ptr = mapping.ptr() as *mut u8;
                let len = mapping.length();
                let copy_len = ((w * h * 4) as usize).min(len);
                for i in (0..copy_len).step_by(4) {
                    unsafe {
                        let r = *data.as_ptr().add(i);
                        let b = *data.as_ptr().add(i + 2);
                        *ptr.add(i) = b;
                        *ptr.add(i + 1) = *data.as_ptr().add(i + 1);
                        *ptr.add(i + 2) = r;
                        *ptr.add(i + 3) = *data.as_ptr().add(i + 3);
                    }
                }
                drop(mapping);
                let tex = renderer.import_dmabuf(&allocated.dmabuf, None).ok()?;
                let dmabuf = if let Some(images) = tex.egl_images() {
                    if let Some(&image) = images.first() {
                        let display = renderer.egl_context().display();
                        let size = Size::from((w as i32, h as i32));
                        display.create_dmabuf_from_image(image, size, tex.is_y_inverted()).ok()
                    } else { None }
                } else { None };
                Some(((zoom, y, x), tex, dmabuf, w, h))
            }).collect()
        } else {
            loaded.into_iter().filter_map(|(zoom, y, x, data, w, h)| {
                let tex = renderer.import_memory(&data, Fourcc::Abgr8888, Size::from((w as i32, h as i32)), false).ok()?;
                Some(((zoom, y, x), tex, None, w, h))
            }).collect()
        }
    }
}
