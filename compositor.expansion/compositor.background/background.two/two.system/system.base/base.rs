use compositor_background_two_draw_element::element::ParallaxBackground;
use compositor_background_two_state_base::state::Two;
use compositor_support_system_buffer_token_base::y5_buffer;
use compositor_support_system_storage_token_base::base::{Token, TokenMut};
use compositor_support_system_trait_system_base::base::{BufferCx, System, SystemCx, WorldBuilder};
use compositor_support_system_world_frame_base::base::{self as layer, FramePlan, FrameTick};
use smithay::backend::renderer::gles::GlesRenderer;
use std::any::Any;

pub static BG_TWO: Token<Two> = Token::new();
/// TRANSITIONAL pub: lock/capture still mutate the instance directly.
pub static BG_TWO_MUT: TokenMut<Two> = TokenMut::new(&BG_TWO);

enum TwoCmd {
    SetInstance(ParallaxBackground),
    Tick,
    Pan(f32, f32),
    Zoom(f32),
}
y5_buffer!(TWO_BUF: TwoCmd);

/// The 2D parallax background system: `update()` (re)builds the GPU resource
/// via the platform hatch and ticks animation; `draw()` emits the node.
#[derive(Default)]
pub struct TwoSystem;

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
        let stale = state.instance.as_ref().is_some_and(|i| i.output_size != size);
        if state.instance.is_none() || stale {
            if let Some(renderer) = cx
                .platform
                .as_deref_mut()
                .and_then(|p| p.downcast_mut::<compositor_orchestration_draw_platform_base::platform::Platform>())
                .and_then(|p| p.renderer())
            {
                let instance = ParallaxBackground::new(renderer, size);
                cx.write(&TWO_BUF, TwoCmd::SetInstance(instance));
            }
            return;
        }
        // Advance the parallax animation (mutation -> buffer, honoring read-only update).
        cx.write(&TWO_BUF, TwoCmd::Tick);
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
            TwoCmd::SetInstance(instance) => two.instance = Some(instance),
            TwoCmd::Tick => { if let Some(i) = &mut two.instance { i.update(); } }
            TwoCmd::Pan(x, y) => { if let Some(i) = &mut two.instance { i.pan = (x, y); } }
            TwoCmd::Zoom(z) => { if let Some(i) = &mut two.instance { i.zoom = z; } }
        }
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
