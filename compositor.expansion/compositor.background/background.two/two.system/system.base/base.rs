use compositor_background_two_draw_element::element::{ParallaxBackground, WallpaperTile};
use compositor_background_two_state_base::state::Two;
use compositor_support_system_buffer_token_base::y5_buffer;
use compositor_support_system_trait_system_base::base::{BufferCx, System, SystemCx, WorldBuilder};
use compositor_support_system_world_frame_base::base::{self as layer, FramePlan, FrameTick};
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

// The per-world background slot tokens live in `two.storage`; the system reads
// and writes that slot in update/draw/buffer.
use compositor_background_two_storage_base::base::{BG_TWO, BG_TWO_MUT};

enum TwoCmd {
    SetInstance(ParallaxBackground),
    SetWallpaperTiles(Vec<WallpaperTile>),
    Tick,
    Pan(f32, f32),
    Zoom(f32),
    /// New output size — applied IN PLACE (keeps `start_time`/`commit`), never a
    /// recreate. See the size-change note in `update()`.
    Resize(f32, f32),
}
y5_buffer!(TWO_BUF: TwoCmd);

/// Detected info from a vips dzsave --layout google tile pyramid.
struct WallpaperInfo {
    /// Padded image dimensions in world pixels (tile_count_at_max_zoom × 256).
    size: (u32, u32),
    /// Highest zoom level directory found under tiles/.
    max_zoom: u32,
}

/// The 2D parallax background system: `update()` (re)builds the GPU resource
/// via the platform hatch, ticks animation, and loads wallpaper tiles when a
/// wallpaper image is configured. `draw()` emits the node.
pub struct TwoSystem {
    /// LRU cache of loaded tile textures keyed by `(zoom, y, x)`.
    /// Value is (texture, tex_w, tex_h) — actual texture dimensions.
    tile_cache: LruCache<(u32, i32, i32), (GlesTexture, u32, u32)>,
    /// Dmabufs for each tile, keyed by `(zoom, y, x)`. Must be kept alive
    /// as long as the texture is used for Vulkan import.
    dmabufs: HashMap<(u32, i32, i32), Dmabuf>,
    /// The wallpaper path we last loaded tiles for. `None` = no wallpaper or
    /// first run. Compared each frame to detect path changes.
    last_wallpaper: Option<String>,
    /// Cached wallpaper tile-pyramid info, cleared when the path changes.
    /// `None` = no wallpaper or detection hasn't run yet.
    info: Option<WallpaperInfo>,
    /// Last camera pan position for damage tracking.
    last_pan: (f32, f32),
    /// Last camera zoom level for damage tracking.
    last_zoom: f32,
    /// Cached visible tile list (rebuilt only when camera moves).
    cached_tiles: Vec<WallpaperTile>,
}

impl Default for TwoSystem {
    fn default() -> Self {
        Self {
            tile_cache: LruCache::new(NonZeroUsize::new(256).unwrap()),
            dmabufs: HashMap::new(),
            last_wallpaper: None,
            info: None,
            last_pan: (0.0, 0.0),
            last_zoom: 1.0,
            cached_tiles: Vec::new(),
        }
    }
}

impl System for TwoSystem {
    fn name(&self) -> &'static str {
        "background.two"
    }

    fn register(&mut self, builder: &mut WorldBuilder) {
        builder.storage.insert(&BG_TWO, Two::new());
        builder.receive(&compositor_y5_camera_system_base::base::CAMERA_MOVED, Self::on_camera_moved);
        builder.receive(&compositor_y5_camera_system_base::base::CAMERA_ZOOMED, Self::on_camera_zoomed);
    }

    fn update(&mut self, cx: &mut SystemCx, _tick: &FrameTick) {
        // bevy lock-morph gate; absent BG_THREE (test worlds) = not locked.
        if cx.storage.try_get(&compositor_background_three_system_base::base::BG_THREE)
            .is_some_and(|b| b.example_lock_done) { return; }
        // Physical output size from the per-frame screen driver-data (set by the
        // frame driver before systems run) — no background-private size token.
        let size = cx.kernel.get(&compositor_orchestration_smithay_data_base::data::SCREEN).size;
        let size = (size.w as f32, size.h as f32);
        let state = cx.storage.get(&BG_TWO);
        // A size change must NOT recreate the instance. With multiple outputs of
        // differing sizes this `update()` runs once PER OUTPUT, each with that
        // output's `SCREEN` size, so a size-triggered rebuild would fire every
        // frame — resetting `start_time` (freezing the shader clock at ~0) and the
        // `commit` counter (no damage → the per-frame reschedule dies and the
        // parallax stops animating). Only a MISSING instance forces a full rebuild
        // (shader/params edits null the slot from the rim); a size change resizes
        // IN PLACE below (the shader is size-independent — `build()` ignores size,
        // and `draw()`/`bind_pane` use the actual per-pane `dst` size).
        let stale = state.instance.as_ref().is_some_and(|i| i.output_size != size);
        let rebuild = state.instance.is_none();
        // Resolve once: this world's override → preference default → built-in.
        // (Setting `instance = None` from the rim forces a rebuild on change.)
        let override_sel = state.background_shader.clone();
        let params = state.params.clone();
        let invert_pan_x = state.invert_pan_x;
        let invert_pan_y = state.invert_pan_y;
        let srgb = state.srgb;
        if rebuild {
            if let Some(renderer) = cx
                .platform
                .as_deref_mut()
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
        // Keep the instance's own size current (used by the non-pane overview /
        // full-screen draw's `geometry()` damage rect) WITHOUT recreating it — each
        // output's `update()`+draw run in the same prepare, so this hands the frame
        // its output's size while the animation clock and commit counter survive.
        if stale {
            cx.write(&TWO_BUF, TwoCmd::Resize(size.0, size.1));
        }
        // Advance the parallax animation (mutation -> buffer, honoring read-only update).
        cx.write(&TWO_BUF, TwoCmd::Tick);

        // --- Wallpaper tile loading --------------------------------------------
        self.load_wallpaper_tiles(state, cx, size);
    }

    fn draw(&mut self, cx: &mut SystemCx, plan: &mut FramePlan) {
        if cx.storage.try_get(&compositor_background_three_system_base::base::BG_THREE)
            .is_some_and(|b| b.example_lock_done) { return; }
        // Renderer-agnostic node; the frame driver bridges + lowers it.
        if let Some(instance) = &cx.storage.get(&BG_TWO).instance {
            plan.push(layer::BACKGROUND, Box::new(instance.clone()));
        }
    }

    fn buffer(&mut self, cx: &mut BufferCx, message: Box<dyn Any>) {
        let two = cx.storage.get_mut(&BG_TWO_MUT);
        match *message.downcast::<TwoCmd>().expect("two buffer type") {
            TwoCmd::SetInstance(instance) => {
                two.shader_error = instance.shader_error.clone();
                two.instance = Some(instance);
            }
            TwoCmd::SetWallpaperTiles(tiles) => {
                if let Some(i) = &mut two.instance {
                    i.set_wallpaper_tiles(tiles);
                }
            }
            TwoCmd::Tick => { if let Some(i) = &mut two.instance { i.update(); } }
            TwoCmd::Pan(x, y) => { if let Some(i) = &mut two.instance { i.pan = (x, y); } }
            TwoCmd::Zoom(z) => { if let Some(i) = &mut two.instance { i.zoom = z; } }
            TwoCmd::Resize(w, h) => { if let Some(i) = &mut two.instance { i.output_size = (w, h); } }
        }
    }

    /// Persist this world's background selection + variable overrides into a
    /// single per-world file `<world>/world.background.json`, rehydrated into the
    /// `BG_TWO` slot at world build.
    fn persist(
        &self,
    ) -> &'static [&'static compositor_support_system_persist_entry_base::base::PersistEntry] {
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

    /// Load visible wallpaper tiles from disk, upload to GL, and send a
    /// `SetWallpaperTiles` command. Clears the tile cache when the wallpaper
    /// path changes. Sends an empty vec when there is no wallpaper (so the
    /// background falls back to the parallax shader).
    /// Uses damage tracking: only rebuilds tile list when camera moves.
    fn load_wallpaper_tiles(&mut self, state: &Two, cx: &mut SystemCx, size: (f32, f32)) {
        let wallpaper_path = match &state.wallpaper_path {
            Some(p) if !p.is_empty() => p.clone(),
            _ => {
                // No wallpaper: clear tiles if we previously had some.
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

        // Detect path change — re-scan the dzsave directory and clear stale cache.
        if self.last_wallpaper.as_ref() != Some(&wallpaper_path) {
            self.last_wallpaper = Some(wallpaper_path.clone());
            self.info = self.detect_wallpaper_info(&wallpaper_path);
            self.tile_cache.clear();
            self.cached_tiles.clear();
        }

        // Get the renderer for GL texture upload.
        let renderer = match cx
            .platform
            .as_deref_mut()
            .and_then(|p| p.downcast_mut::<compositor_orchestration_draw_platform_base::platform::Platform>())
            .and_then(|p| p.renderer())
        {
            Some(r) => r,
            None => return,
        };

        // Compute visible tiles from current camera state.
        let instance = match &state.instance {
            Some(i) => i,
            None => return,
        };

        // Damage tracking: only rebuild tile list when camera moves.
        let camera_changed = (instance.pan.0 - self.last_pan.0).abs() > 0.1
            || (instance.pan.1 - self.last_pan.1).abs() > 0.1
            || (instance.zoom - self.last_zoom).abs() > 0.01;

        if camera_changed || self.cached_tiles.is_empty() {
            self.last_pan = instance.pan;
            self.last_zoom = instance.zoom;

            // Use dmabuf for Vulkan renderer, import_memory for GLES.
            let use_dmabuf = compositor_developer_environment_config_base::base::get()
                .renderer == "vulkan";
            self.cached_tiles = self.compute_visible_tiles(
                renderer, &wallpaper_path, instance.pan, instance.zoom,
                instance.output_size, use_dmabuf,
            );
        }

        cx.write(&TWO_BUF, TwoCmd::SetWallpaperTiles(self.cached_tiles.clone()));
    }

    /// Scan the dzsave tile directory to detect image dimensions and available
    /// zoom levels. Returns `None` when the directory structure is absent or
    /// unrecognisable.
    fn detect_wallpaper_info(&self, wallpaper_path: &str) -> Option<WallpaperInfo> {
        let tiles_dir = Path::new(wallpaper_path).join("tiles");
        if !tiles_dir.is_dir() {
            return None;
        }
        // Find the highest zoom level directory (e.g. tiles/0/, tiles/1/, …).
        let max_zoom = (0u32..30)
            .filter(|z| tiles_dir.join(z.to_string()).is_dir())
            .max()?;
        // Count tiles at the max zoom: find the highest y dir, then the
        // highest-numbered x file in that dir (try .png then .jpg).
        let zoom_dir = tiles_dir.join(max_zoom.to_string());
        let max_y = (0i32..10000)
            .find(|&y| !zoom_dir.join(y.to_string()).is_dir())
            .unwrap_or(0);
        if max_y == 0 {
            return None;
        }
        let last_y_dir = zoom_dir.join((max_y - 1).to_string());
        let has_png = last_y_dir.join("0.png").exists();
        let has_jpg = !has_png && last_y_dir.join("0.jpg").exists();
        if !has_png && !has_jpg {
            return None;
        }
        let ext = if has_png { "png" } else { "jpg" };
        let max_x = (0i32..10000)
            .find(|&x| !last_y_dir.join(format!("{x}.{ext}")).exists())
            .unwrap_or(1);
        Some(WallpaperInfo {
            size: (max_x.max(1) as u32 * 256, max_y.max(1) as u32 * 256),
            max_zoom,
        })
    }

    /// Determine which tiles are visible in the current viewport, load them
    /// from disk (or cache), and return the list of `WallpaperTile`s.
    ///
    /// The wallpaper image is placed in world space centred at the origin
    /// with its pixel dimensions as world units at zoom=1.0.  The standard
    /// y5 camera transform projects tiles to screen coordinates:
    ///
    ///   screen_x = (world_x - pan.x) × zoom + output_w/2
    ///
    /// The tile-zoom level is chosen so that each tile's screen size stays
    /// near 256 px at the current camera zoom.
    ///
    /// When `use_dmabuf` is true, tiles are loaded via GBM dmabuf for
    /// Vulkan zero-copy rendering. When false, tiles use import_memory.
    fn compute_visible_tiles(
        &mut self,
        renderer: &mut GlesRenderer,
        wallpaper_path: &str,
        pan: (f32, f32),
        zoom: f32,
        output_size: (f32, f32),
        use_dmabuf: bool,
    ) -> Vec<WallpaperTile> {
        const TILE_PX: f32 = 256.0;
        let info = match &self.info {
            Some(i) => i,
            None => return Vec::new(),
        };
        let (img_w, img_h) = (info.size.0 as f32, info.size.1 as f32);
        let max_z = info.max_zoom;

        // --- Tile zoom selection ------------------------------------------------
        // At zoom level z, one tile covers TILE_PX × 2^(max_z − z) world pixels.
        // Its screen size is world_size × zoom.  We want screen_size ≈ TILE_PX.
        //   TILE_PX × 2^(max_z − z) × zoom = TILE_PX
        //   ⇒  z = max_z + log2(zoom)
        let tile_zoom = (max_z as f32 + zoom.log2())
            .clamp(0.0, max_z as f32) as u32;

        // --- Number of tiles at this zoom level ---------------------------------
        // At level z each tile grid cell covers 256 × 2^(max_z−z) world pixels.
        let step = 1u32 << (max_z - tile_zoom); // 2^(max_z − tile_zoom)
        let cell_world = TILE_PX * step as f32;
        let num_tiles_x = ((img_w + cell_world - 1.0) / cell_world) as u32;
        let num_tiles_y = ((img_h + cell_world - 1.0) / cell_world) as u32;
        let num_tiles_x = num_tiles_x.max(1);
        let num_tiles_y = num_tiles_y.max(1);

        // Per-tile world dimensions (fraction of the image).
        let tile_world_w = img_w / num_tiles_x as f32;
        let tile_world_h = img_h / num_tiles_y as f32;

        // --- Viewport in world coordinates --------------------------------------
        // The output rect [0, 0, w, h] in screen space mapped to world space.
        let vp_left   = pan.0 - output_size.0 / (2.0 * zoom);
        let vp_right  = pan.0 + output_size.0 / (2.0 * zoom);
        let vp_top    = pan.1 - output_size.1 / (2.0 * zoom);
        let vp_bottom = pan.1 + output_size.1 / (2.0 * zoom);

        // Shift by img/2 to get image-local coordinates (0 = left/top edge).
        let vp_left_img   = vp_left   + img_w / 2.0;
        let vp_right_img  = vp_right  + img_w / 2.0;
        let vp_top_img    = vp_top    + img_h / 2.0;
        let vp_bottom_img = vp_bottom + img_h / 2.0;

        // --- Visible tile range in image-local coordinates ----------------------
        let tx_min = (vp_left_img   / tile_world_w).floor().max(0.0) as u32;
        let tx_max = ((vp_right_img / tile_world_w).ceil() as u32)
            .saturating_sub(1).min(num_tiles_x - 1);
        let ty_min = (vp_top_img    / tile_world_h).floor().max(0.0) as u32;
        let ty_max = ((vp_bottom_img / tile_world_h).ceil() as u32)
            .saturating_sub(1).min(num_tiles_y - 1);

        // --- Load visible tiles ------------------------------------------------
        let tiles_dir = Path::new(wallpaper_path).join("tiles");
        let mut result = Vec::new();
        let mut missing_tiles: Vec<(u32, u32, u32, u32)> = Vec::new();

        // Phase 1: Determine which tiles are needed (check cache first).
        for ty in ty_min..=ty_max {
            for tx in tx_min..=tx_max {
                // Find the best ancestor tile (cached or to-be-loaded).
                let mut best_key: Option<(u32, i32, i32)> = None;

                for fallback_z in (0..=tile_zoom).rev() {
                    let d = tile_zoom - fallback_z;
                    let ancestor_x = tx >> d;
                    let ancestor_y = ty >> d;
                    let key = (fallback_z, ancestor_y as i32, ancestor_x as i32);

                    if self.tile_cache.contains(&key) {
                        best_key = Some(key);
                        break;
                    }
                    // Record the first missing tile for batch loading.
                    if best_key.is_none() {
                        best_key = Some(key);
                        missing_tiles.push((fallback_z, ancestor_x, ancestor_y, d));
                        break;
                    }
                }
            }
        }

        // Phase 2: Batch-load missing tiles from disk in parallel.
        if !missing_tiles.is_empty() {
            let loaded = Self::batch_load_tiles(renderer, &tiles_dir, &missing_tiles, use_dmabuf);
            for (key, tex, dmabuf, tex_w, tex_h) in loaded {
                self.tile_cache.put(key, (tex, tex_w, tex_h));
                if let Some(dm) = dmabuf {
                    self.dmabufs.insert(key, dm);
                }
            }
        }

        // Phase 3: Build the visible tile list from cache.
        for ty in ty_min..=ty_max {
            for tx in tx_min..=tx_max {
                let mut found: Option<(GlesTexture, u32, u32)> = None;
                let mut found_key: Option<(u32, i32, i32)> = None;

                for fallback_z in (0..=tile_zoom).rev() {
                    let d = tile_zoom - fallback_z;
                    let ancestor_x = tx >> d;
                    let ancestor_y = ty >> d;
                    let key = (fallback_z, ancestor_y as i32, ancestor_x as i32);

                    if let Some(entry) = self.tile_cache.get(&key).cloned() {
                        found = Some(entry);
                        found_key = Some(key);
                        break;
                    }
                }

                let (texture, tex_w, tex_h) = match found {
                    Some(entry) => entry,
                    None => continue,
                };

                // Tile's world-space top-left (image centred at world origin).
                let tile_left = (img_w * tx as f32 / num_tiles_x as f32) - img_w / 2.0;
                let tile_top  = (img_h * ty as f32 / num_tiles_y as f32) - img_h / 2.0;

                let sx = (tile_left - pan.0) * zoom + output_size.0 / 2.0;
                let sy = (tile_top  - pan.1) * zoom + output_size.1 / 2.0;
                let sx = sx.round();
                let sy = sy.round();
                let sw = (tile_world_w * zoom).ceil();
                let sh = (tile_world_h * zoom).ceil();

                if sx + sw <= 0.0 || sx >= output_size.0
                    || sy + sh <= 0.0 || sy >= output_size.1
                {
                    continue;
                }

                // Use cached dmabuf (exported during batch_load_tiles).
                let dmabuf = if use_dmabuf {
                    found_key.and_then(|key| self.dmabufs.get(&key).cloned())
                } else {
                    None
                };

                result.push(WallpaperTile {
                    x: sx as i32, y: sy as i32,
                    w: sw as i32, h: sh as i32,
                    tex_w,
                    tex_h,
                    texture,
                    dmabuf,
                });
            }
        }

        // --- Prefetch tiles at zoom+1 for center 25% of viewport ----------------
        // Pre-load higher-resolution tiles that will be needed soon, reducing
        // pop-in when the user zooms in further.
        if tile_zoom < max_z {
            let prefetch_zoom = tile_zoom + 1;
            let prefetch_step = 1u32 << (max_z - prefetch_zoom);
            let prefetch_cell_world = TILE_PX * prefetch_step as f32;
            let prefetch_num_tiles_x = ((img_w + prefetch_cell_world - 1.0) / prefetch_cell_world).max(1.0) as u32;
            let prefetch_num_tiles_y = ((img_h + prefetch_cell_world - 1.0) / prefetch_cell_world).max(1.0) as u32;

            // Centre 25% of the viewport in world coordinates.
            let margin = 0.25;
            let vp_cx = pan.0;
            let vp_cy = pan.1;
            let half_w = output_size.0 / (2.0 * zoom) * margin;
            let half_h = output_size.1 / (2.0 * zoom) * margin;

            let vp_left_img   = vp_cx - half_w + img_w / 2.0;
            let vp_right_img  = vp_cx + half_w + img_w / 2.0;
            let vp_top_img    = vp_cy - half_h + img_h / 2.0;
            let vp_bottom_img = vp_cy + half_h + img_h / 2.0;

            let px_min = (vp_left_img   / prefetch_cell_world).floor().max(0.0) as u32;
            let px_max = ((vp_right_img / prefetch_cell_world).ceil() as u32)
                .saturating_sub(1).min(prefetch_num_tiles_x - 1);
            let py_min = (vp_top_img    / prefetch_cell_world).floor().max(0.0) as u32;
            let py_max = ((vp_bottom_img / prefetch_cell_world).ceil() as u32)
                .saturating_sub(1).min(prefetch_num_tiles_y - 1);

            // Collect missing prefetch tiles for parallel loading.
            let mut prefetch_missing: Vec<(u32, u32, u32, u32)> = Vec::new();
            for py in py_min..=py_max {
                for px in px_min..=px_max {
                    let key = (prefetch_zoom, py as i32, px as i32);
                    if !self.tile_cache.contains(&key) {
                        prefetch_missing.push((prefetch_zoom, px, py, 0));
                    }
                }
            }

            // Batch-load prefetch tiles in parallel.
            if !prefetch_missing.is_empty() {
                let loaded = Self::batch_load_tiles(renderer, &tiles_dir, &prefetch_missing, use_dmabuf);
                for (key, tex, dmabuf, tex_w, tex_h) in loaded {
                    self.tile_cache.put(key, (tex, tex_w, tex_h));
                    if let Some(dm) = dmabuf {
                        self.dmabufs.insert(key, dm);
                    }
                }
            }
        }

        result
    }

    /// Load a single tile from a PNG/JPEG file and upload it as a GLES texture.
    /// Returns the texture or an error if the tile doesn't exist or decoding fails.
    fn load_tile_texture(
        renderer: &mut GlesRenderer,
        path: &Path,
    ) -> Result<GlesTexture, Box<dyn std::error::Error>> {
        let img = image::open(path)?.into_rgba8();
        let (w, h) = img.dimensions();
        let data = img.into_raw();
        // image crate produces RGBA (R at byte 0) → Fourcc::Abgr8888 maps to GL_RGBA8.
        let tex = renderer.import_memory(
            &data,
            Fourcc::Abgr8888,
            Size::from((w as i32, h as i32)),
            false,
        )?;
        Ok(tex)
    }

    /// Load tile image data from disk without uploading to GPU.
    /// This can be called from parallel threads safely.
    fn load_tile_image(path: &Path) -> Option<(Vec<u8>, u32, u32)> {
        let img = image::open(path).ok()?.into_rgba8();
        let (w, h) = img.dimensions();
        Some((img.into_raw(), w, h))
    }

    /// Batch-load missing tiles from disk in parallel, then upload to GPU.
    /// Returns (key, texture, optional_dmabuf, tex_w, tex_h) for each loaded tile.
    /// When `use_dmabuf` is true, uses GBM dmabuf for Vulkan compatibility.
    /// When false, uses import_memory for GLES.
    fn batch_load_tiles(
        renderer: &mut GlesRenderer,
        tiles_dir: &Path,
        missing: &[(u32, u32, u32, u32)], // (zoom, x, y, ancestor_offset)
        use_dmabuf: bool,
    ) -> Vec<((u32, i32, i32), GlesTexture, Option<Dmabuf>, u32, u32)> {
        use rayon::prelude::*;
        use compositor_support_bevy_core_alloc_base::allocate_dmabuf;
        use smithay::backend::allocator::Buffer;
        use smithay::backend::allocator::dmabuf::DmabufMappingMode;
        use smithay::backend::renderer::ImportDma;

        // Phase 1: Parallel CPU load (image decode only, no GPU).
        let loaded: Vec<_> = missing
            .par_iter()
            .filter_map(|&(zoom, x, y, _)| {
                let base = tiles_dir.join(zoom.to_string()).join(y.to_string());
                let path_png = base.join(format!("{x}.png"));
                let path_jpg = base.join(format!("{x}.jpg"));
                let (data, w, h) = Self::load_tile_image(&path_png)
                    .or_else(|| Self::load_tile_image(&path_jpg))?;
                Some((zoom, y as i32, x as i32, data, w, h))
            })
            .collect();

        // Phase 2: Sequential GPU upload (renderer is !Send).
        if use_dmabuf {
            // Use GBM dmabuf allocation for Vulkan-compatible textures.
            loaded
                .into_iter()
                .filter_map(|(zoom, y, x, data, w, h)| {
                    // Allocate dmabuf via GBM.
                    let allocated = allocate_dmabuf("/dev/dri/renderD128", w, h).ok()?;

                    // Copy image data into dmabuf, swapping R and B channels.
                    // PNG is RGBA (R at byte 0), but GBM Argb8888 is BGRA (B at byte 0).
                    let mapping = allocated.dmabuf.map_plane(0, DmabufMappingMode::READ | DmabufMappingMode::WRITE).ok()?;
                    let ptr = mapping.ptr() as *mut u8;
                    let len = mapping.length();
                    let pixel_count = (w * h) as usize;
                    let copy_len = (pixel_count * 4).min(len);
                    // Swap R (byte 0) and B (byte 2) for each pixel.
                    for i in (0..copy_len).step_by(4) {
                        unsafe {
                            let r = *data.as_ptr().add(i);
                            let b = *data.as_ptr().add(i + 2);
                            *ptr.add(i) = b;      // B -> byte 0
                            *ptr.add(i + 1) = *data.as_ptr().add(i + 1); // G
                            *ptr.add(i + 2) = r;  // R -> byte 2
                            *ptr.add(i + 3) = *data.as_ptr().add(i + 3); // A
                        }
                    }
                    drop(mapping);

                    // Import dmabuf into GLES renderer (creates texture with egl_images).
                    let tex = renderer.import_dmabuf(&allocated.dmabuf, None).ok()?;

                    // Export dmabuf for Vulkan import (done once at load time).
                    let dmabuf = if let Some(images) = tex.egl_images() {
                        if let Some(&image) = images.first() {
                            let display = renderer.egl_context().display();
                            let size = Size::from((w as i32, h as i32));
                            display.create_dmabuf_from_image(image, size, tex.is_y_inverted()).ok()
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    // Return texture AND dmabuf to keep dmabuf alive.
                    Some(((zoom, y, x), tex, dmabuf, w, h))
                })
                .collect()
        } else {
            // Use import_memory for GLES-only rendering.
            loaded
                .into_iter()
                .filter_map(|(zoom, y, x, data, w, h)| {
                    let tex = renderer.import_memory(&data, Fourcc::Abgr8888, Size::from((w as i32, h as i32)), false).ok()?;
                    Some(((zoom, y, x), tex, None, w, h))
                })
                .collect()
        }
    }
}
