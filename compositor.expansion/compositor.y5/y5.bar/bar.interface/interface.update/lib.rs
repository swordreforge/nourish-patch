//! Status bar per-frame updates: clock, battery, show/hide.
//! The handle is shared via `interface.base` statics.
use smithay::utils::Point;
use compositor_monitor_compositor_iced_base::{HandleId, IcedHandle};
use compositor_orchestration_core_state_base::Loop;
use compositor_y5_bar_ui_base::base::{StatusBar, StatusBarMessage, BAR_HEIGHT};
use compositor_y5_bar_interface_base::base;

pub fn show(state: &mut Loop) {
    if base::is_visible() { return; }
    let raw = base::handle_raw();
    if raw == 0 { return; }
    if let Some(reg) = state.inner.surface_mut().registry.as_mut() {
        reg.set_location_by_id(HandleId(raw), Point::new(0, 0));
    }
    base::set_visible(true);
}

pub fn hide(state: &mut Loop) {
    if !base::is_visible() { return; }
    let raw = base::handle_raw();
    if raw == 0 { return; }
    if let Some(reg) = state.inner.surface_mut().registry.as_mut() {
        reg.set_location_by_id(HandleId(raw), Point::new(0, -BAR_HEIGHT));
    }
    base::set_visible(false);
}

pub fn update_clock(state: &mut Loop, label: String) {
    let raw = base::handle_raw();
    if raw == 0 { return; }
    if let Some(reg) = state.inner.surface_mut().registry.as_mut() {
        let _ = reg.dispatch_message(
            IcedHandle::<StatusBar>::from_id(HandleId(raw)),
            StatusBarMessage::Clock(label),
        );
    }
}

pub fn update_battery(state: &mut Loop, label: Option<String>) {
    let raw = base::handle_raw();
    if raw == 0 { return; }
    if let Some(reg) = state.inner.surface_mut().registry.as_mut() {
        let _ = reg.dispatch_message(
            IcedHandle::<StatusBar>::from_id(HandleId(raw)),
            StatusBarMessage::Battery(label),
        );
    }
}
