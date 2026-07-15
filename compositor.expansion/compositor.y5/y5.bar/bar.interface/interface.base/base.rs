use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU16, Ordering};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::utils::{Physical, Point, Rectangle, Size};
use compositor_monitor_compositor_iced_base::HandleId;
use compositor_orchestration_core_state_base::Loop;
use compositor_y5_bar_ui_base::base::{StatusBar, BAR_HEIGHT};

static BAR_HANDLE: AtomicU64 = AtomicU64::new(0);
static BAR_VISIBLE: AtomicBool = AtomicBool::new(true);
static HIDE_GRACE: AtomicU16 = AtomicU16::new(0);

const SHOW_Y: f64 = 4.0;
const HIDE_Y: f64 = 40.0;
const HIDE_FRAMES: u16 = 15;

pub fn handle_raw() -> u64 { BAR_HANDLE.load(Ordering::Acquire) }
pub fn is_visible() -> bool { BAR_VISIBLE.load(Ordering::Acquire) }
pub fn set_visible(v: bool) { BAR_VISIBLE.store(v, Ordering::Release); }
pub fn set_handle(h: u64) { BAR_HANDLE.store(h, Ordering::Release); }

pub fn frame_hook(state: &mut Loop, gles: &mut GlesRenderer, _size: Size<i32, Physical>) {
    if BAR_HANDLE.load(Ordering::Acquire) == 0 {
        create(state, gles);
    }
    auto_show_hide(state);
}

fn auto_show_hide(state: &mut Loop) {
    let raw = BAR_HANDLE.load(Ordering::Acquire);
    if raw == 0 { return; }
    let y = state.state.seat.seat.get_pointer()
        .map(|p| p.current_location().y)
        .unwrap_or(f64::NEG_INFINITY);
    let reg = state.inner.surface_mut().registry.as_mut();
    if y <= SHOW_Y {
        HIDE_GRACE.store(0, Ordering::Release);
        if !BAR_VISIBLE.load(Ordering::Acquire) {
            if let Some(reg) = reg { reg.set_location_by_id(HandleId(raw), Point::new(0, 0)); }
            BAR_VISIBLE.store(true, Ordering::Release);
        }
    } else if y > HIDE_Y && BAR_VISIBLE.load(Ordering::Acquire) {
        let grace = HIDE_GRACE.load(Ordering::Acquire) + 1;
        if grace >= HIDE_FRAMES {
            if let Some(reg) = reg { reg.set_location_by_id(HandleId(raw), Point::new(0, -BAR_HEIGHT)); }
            BAR_VISIBLE.store(false, Ordering::Release);
            HIDE_GRACE.store(0, Ordering::Release);
        } else {
            HIDE_GRACE.store(grace, Ordering::Release);
        }
    } else if y <= HIDE_Y {
        HIDE_GRACE.store(0, Ordering::Release);
    }
}

fn create(state: &mut Loop, gles: &mut GlesRenderer) {
    let output = state.inner.active_output();
    let mode = output.current_mode().unwrap_or_else(|| abort!("output has a mode"));
    let rect = Rectangle::new(Point::new(0, 0), Size::new(mode.size.w, BAR_HEIGHT));
    let handle = compositor_y5_surface_draw_handle::handle::load(
        state, gles, StatusBar::new(), rect,
        compositor_y5_surface_draw_handle::handle::IcedSpace::Screen,
        compositor_orchestration_draw_layer_base::base::Layer::SCENE.bits(),
    );
    BAR_HANDLE.store(handle.id.raw(), Ordering::Release);
    BAR_VISIBLE.store(true, Ordering::Release);
    info!("status bar created (id {})", handle.id);
}

pub fn destroy(state: &mut Loop) {
    let raw = BAR_HANDLE.swap(0, Ordering::AcqRel);
    if raw == 0 { return; }
    if let Some(reg) = state.inner.surface_mut().registry.as_mut() {
        reg.destroy_by_id(HandleId(raw));
    }
}

pub fn resize(state: &mut Loop, width: i32) {
    let raw = BAR_HANDLE.load(Ordering::Acquire);
    if raw == 0 { return; }
    if let Some(reg) = state.inner.surface_mut().registry.as_mut() {
        let _ = reg.request_resize_by_id(HandleId(raw), Size::from((width, BAR_HEIGHT)));
    }
}
