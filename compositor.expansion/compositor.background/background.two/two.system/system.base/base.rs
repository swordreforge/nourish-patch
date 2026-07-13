use compositor_background_two_draw_element::element::ParallaxBackground;
use compositor_background_two_draw_wallpaper::{build_or_reuse_cache, prepare_tiles, FillMapping, WallpaperGpuCache, TileBlit};
use compositor_background_two_state_base::state::{Two, WallpaperFillRaw};
use compositor_support_system_buffer_token_base::y5_buffer;
use compositor_support_system_trait_system_base::base::{BufferCx, System, SystemCx, WorldBuilder};
use compositor_support_system_world_frame_base::base::{self as layer, FramePlan, FrameTick};
use smithay::backend::renderer::gles::GlesRenderer;
use std::any::Any;

use compositor_background_two_storage_base::base::{BG_TWO, BG_TWO_MUT};

enum TwoCmd {
    SetInstance(ParallaxBackground),
    Tick,
    Pan(f32, f32),
    Zoom(f32),
    Resize(f32, f32),
    WallpaperTiles(Vec<TileBlit>),
}
y5_buffer!(TWO_BUF: TwoCmd);

/// The 2D parallax background system: `update()` (re)builds the GPU resource
/// via the platform hatch and ticks animation; `draw()` emits the node.
/// Wallpaper GPU cache lives here (system-local), keyed to the current path.
pub struct TwoSystem {
    pub wallpaper_cache: Option<WallpaperGpuCache>,
}

impl Default for TwoSystem {
    fn default() -> Self { Self { wallpaper_cache: None } }
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
        if cx.storage.try_get(&compositor_background_three_system_base::base::BG_THREE)
            .is_some_and(|b| b.example_lock_done) { return; }
        let size = cx.kernel.get(&compositor_orchestration_smithay_data_base::data::SCREEN).size;
        let size = (size.w as f32, size.h as f32);
        let (screen_w, screen_h) = (size.0 as f64, size.1 as f64);
        let state = cx.storage.get(&BG_TWO);
        trace!("TwoSystem::update: rebuild={} wallpaper_path={:?}", state.instance.is_none(), state.wallpaper_path);
        let stale = state.instance.as_ref().is_some_and(|i| i.output_size != size);
        let rebuild = state.instance.is_none();
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
        if stale {
            cx.write(&TWO_BUF, TwoCmd::Resize(size.0, size.1));
        }
        cx.write(&TWO_BUF, TwoCmd::Tick);

        // --- Wallpaper: build / reuse GPU cache from the current path ---
        if let Some(renderer) = cx
            .platform
            .as_deref_mut()
            .and_then(|p| p.downcast_mut::<compositor_orchestration_draw_platform_base::platform::Platform>())
            .and_then(|p| p.renderer())
        {
            let wallpaper_path = state.wallpaper_path.clone();
            let path_is_set = wallpaper_path.is_some();
            trace!("TwoSystem::wallpaper: path={:?} cache_exists={}", wallpaper_path, self.wallpaper_cache.is_some());

            // Rebuild the cache when the path changes (load_or_generate is idempotent on disk).
            if path_is_set {
                let cache = build_or_reuse_cache(
                    wallpaper_path.as_deref(),
                    self.wallpaper_cache.as_mut(),
                    renderer,
                );
                if let Some(new_cache) = cache {
                    info!("TwoSystem::wallpaper: cache built, {} levels", new_cache.index.levels.len());
                    self.wallpaper_cache = Some(new_cache);
                } else if self.wallpaper_cache.is_none() {
                    warn!("TwoSystem::wallpaper: cache build FAILED for {:?}", wallpaper_path);
                }
            } else {
                self.wallpaper_cache = None;
            }

            // Prepare per-frame tile blits when wallpaper is active.
            if let Some(cache) = &mut self.wallpaper_cache {
                let pan = state.instance.as_ref().map(|i| i.pan).unwrap_or((0.0, 0.0));
                let zoom = state.instance.as_ref().map(|i| i.zoom).unwrap_or(1.0);
                let fm = compute_fill_mapping(state.wallpaper_fill, cache, screen_w, screen_h);
                let blits = prepare_tiles(cache, renderer, pan, zoom, size, fm);
                trace!("TwoSystem::wallpaper: {} tiles", blits.len());
                cx.write(&TWO_BUF, TwoCmd::WallpaperTiles(blits));
            } else if !path_is_set {
                cx.write(&TWO_BUF, TwoCmd::WallpaperTiles(vec![]));
            }
        }
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
            TwoCmd::SetInstance(instance) => {
                two.shader_error = instance.shader_error.clone();
                two.instance = Some(instance);
            }
            TwoCmd::Tick => { if let Some(i) = &mut two.instance { i.update(); } }
            TwoCmd::Pan(x, y) => { if let Some(i) = &mut two.instance { i.pan = (x, y); } }
            TwoCmd::Zoom(z) => { if let Some(i) = &mut two.instance { i.zoom = z; } }
            TwoCmd::Resize(w, h) => { if let Some(i) = &mut two.instance { i.output_size = (w, h); } }
            TwoCmd::WallpaperTiles(tiles) => {
                if let Some(i) = &mut two.instance {
                    i.wallpaper_tiles = Some(tiles);
                }
            }
        }
    }

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
}

fn compute_fill_mapping(fill: WallpaperFillRaw, cache: &WallpaperGpuCache, sw: f64, sh: f64) -> FillMapping {
    let (iw, ih) = cache.index.levels.first().map(|l| (l.w as f64, l.h as f64)).unwrap_or((512.0, 512.0));
    let (ia, sa) = (iw / ih, sw / sh);
    let (es, ox, oy) = match fill.0 {
        WallpaperFillRaw::COVER => { let s = if ia > sa { sh / ih } else { sw / iw }; (s, 0.0, 0.0) }
        WallpaperFillRaw::FIT => { let s = if ia > sa { sw / iw } else { sh / ih }; (s, 0.0, 0.0) }
        WallpaperFillRaw::CENTER => (1.0, -(sw - iw) / 2.0, -(sh - ih) / 2.0),
        _ => (1.0, 0.0, 0.0), // Tile
    };
    FillMapping(es, ox, oy)
}
