use compositor_background_two_draw_element::element::ParallaxBackground;
use compositor_background_two_draw_wallpaper::{open_wallpaper, prepare_tiles, FillMapping, WallpaperGpuCache, TileBlit};
use compositor_background_two_state_base::state::{Two, WallpaperFillRaw};
use compositor_support_system_buffer_token_base::y5_buffer;
use compositor_support_system_trait_system_base::base::{BufferCx, System, SystemCx, WorldBuilder};
use compositor_support_system_world_frame_base::base::{self as layer, FramePlan, FrameTick};
use smithay::backend::renderer::gles::GlesRenderer;
use std::any::Any;
use std::path::PathBuf;

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

pub struct TwoSystem {
    pub wallpaper_cache: Option<WallpaperGpuCache>,
    last_pan: (f32, f32),
    last_zoom: f32,
    last_size: (f32, f32),
    cached_blits: Vec<TileBlit>,
}

impl Default for TwoSystem {
    fn default() -> Self {
        Self { wallpaper_cache: None, last_pan: (0.0, 0.0), last_zoom: 1.0, last_size: (0.0, 0.0), cached_blits: Vec::new() }
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
        let (screen_w, screen_h) = (size.0 as f64, size.1 as f64);
        let state = cx.storage.get(&BG_TWO);
        let stale = state.instance.as_ref().is_some_and(|i| i.output_size != size);
        let rebuild = state.instance.is_none();
        let override_sel = state.background_shader.clone();
        let params = state.params.clone();
        let invert_pan_x = state.invert_pan_x;
        let invert_pan_y = state.invert_pan_y;
        let srgb = state.srgb;
        if rebuild {
            if let Some(renderer) = cx.platform.as_deref_mut()
                .and_then(|p| p.downcast_mut::<compositor_orchestration_draw_platform_base::platform::Platform>())
                .and_then(|p| p.renderer())
            {
                let sel = override_sel.or_else(compositor_developer_stats_registry_base::base::background_shader_default);
                let mut instance = ParallaxBackground::new(renderer, size, sel.as_deref(), &params);
                instance.invert_pan_x = invert_pan_x;
                instance.invert_pan_y = invert_pan_y;
                instance.srgb = srgb;
                cx.write(&TWO_BUF, TwoCmd::SetInstance(instance));
            }
            return;
        }
        if stale { cx.write(&TWO_BUF, TwoCmd::Resize(size.0, size.1)); }
        cx.write(&TWO_BUF, TwoCmd::Tick);

        // --- Wallpaper: open vips source on path change, compute tiles on-demand ---
        let wallpaper_path = state.wallpaper_path.clone();
        let path_is_set = wallpaper_path.is_some();

        // Open vips image when path changes (lazy — no decode yet).
        if path_is_set {
            let needs_open = match &self.wallpaper_cache {
                Some(c) => c.source != PathBuf::from(wallpaper_path.as_deref().unwrap_or("")),
                None => true,
            };
            if needs_open {
                if let Some(path) = &wallpaper_path {
                    if let Some(cache) = open_wallpaper(path) {
                        info!("TwoSystem::wallpaper: opened vips source for {}", path);
                        self.wallpaper_cache = Some(cache);
                        self.cached_blits.clear();
                    }
                }
            }
        } else {
            self.wallpaper_cache = None;
            self.cached_blits.clear();
        }

        // Compute tile blits from vips source on-demand.
        if let Some(renderer) = cx.platform.as_deref_mut()
            .and_then(|p| p.downcast_mut::<compositor_orchestration_draw_platform_base::platform::Platform>())
            .and_then(|p| p.renderer())
        {
            if let Some(cache) = &mut self.wallpaper_cache {
                let pan = state.instance.as_ref().map(|i| i.pan).unwrap_or((0.0, 0.0));
                let zoom = state.instance.as_ref().map(|i| i.zoom).unwrap_or(1.0);
                if pan != self.last_pan || zoom != self.last_zoom || size != self.last_size || self.cached_blits.is_empty() {
                    let fm = compute_fill_mapping(state.wallpaper_fill, cache, screen_w, screen_h);
                    self.cached_blits = prepare_tiles(cache, renderer, pan, zoom, size, fm);
                    self.last_pan = pan; self.last_zoom = zoom; self.last_size = size;
                }
                cx.write(&TWO_BUF, TwoCmd::WallpaperTiles(self.cached_blits.clone()));
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
            TwoCmd::SetInstance(instance) => { two.shader_error = instance.shader_error.clone(); two.instance = Some(instance); }
            TwoCmd::Tick => { if let Some(i) = &mut two.instance { i.update(); } }
            TwoCmd::Pan(x, y) => { if let Some(i) = &mut two.instance { i.pan = (x, y); } }
            TwoCmd::Zoom(z) => { if let Some(i) = &mut two.instance { i.zoom = z; } }
            TwoCmd::Resize(w, h) => { if let Some(i) = &mut two.instance { i.output_size = (w, h); } }
            TwoCmd::WallpaperTiles(tiles) => { if let Some(i) = &mut two.instance { i.wallpaper_tiles = Some(tiles); } }
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
}

fn compute_fill_mapping(fill: WallpaperFillRaw, cache: &WallpaperGpuCache, sw: f64, sh: f64) -> FillMapping {
    let (iw, ih) = (cache.source_w as f64, cache.source_h as f64);
    let (ia, sa) = (iw / ih, sw / sh);
    let (es, ox, oy) = match fill.0 {
        WallpaperFillRaw::COVER => { let s = if ia > sa { sh / ih } else { sw / iw }; (s, 0.0, 0.0) }
        WallpaperFillRaw::FIT => { let s = if ia > sa { sw / iw } else { sh / ih }; (s, 0.0, 0.0) }
        WallpaperFillRaw::CENTER => (1.0, -(sw - iw) / 2.0, -(sh - ih) / 2.0),
        _ => (1.0, 0.0, 0.0),
    };
    FillMapping(es, ox, oy)
}
