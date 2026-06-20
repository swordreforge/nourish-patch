#[macro_use]
extern crate compositor_developer_debug_instance_record;

use smithay::desktop::PopupKind;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use compositor_support_smithay_dispatch_state_base::state::{Dispatch, DispatchWire};

/// Marks a window that has had its initial Space placement applied. Set by
/// `apply_commit` (the world-side drain) so re-commits don't re-place. Lives
/// here because both the drain and the canvas read it.
pub struct WindowPlacedMarker;

/// Popup commit — PROTOCOL only (Dispatch). The toplevel/window placement that
/// used to live here moved to the world-side `apply_commit`
/// (document/SMITHAY_DECOUPLING.md): `commit` must not touch the world.
pub fn handle_commit(
    _loop: &mut Dispatch,
    surface: &WlSurface,
) {
    _loop.popup.state.commit(surface);
    if let Some(popup) = _loop.popup.state.find_popup(surface) {
        match popup {
            PopupKind::Xdg(ref xdg) => {
                if !xdg.is_initial_configure_sent() {
                    xdg.send_configure()
                        .unwrap_or_else(|e| abort!("initial configure failed: {e:?}"));
                }
            }
            PopupKind::InputMethod(ref _input_method) => {}
        }
    }
}
