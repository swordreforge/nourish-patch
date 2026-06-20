use smithay::reexports::wayland_protocols_wlr::layer_shell::v1::server::zwlr_layer_shell_v1::ZwlrLayerShellV1;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::shell::wlr_layer::{WlrLayerShellGlobalData, WlrLayerShellState};
use compositor_support_smithay_dispatch_state_base::state::DispatchWire;
use compositor_support_smithay_state_layershell_base::state::Layershell;

pub fn new<I: DispatchWire>(display_handle: &DisplayHandle) -> Layershell  where
    I: GlobalDispatch<ZwlrLayerShellV1, WlrLayerShellGlobalData>,
    I: 'static,
{
    let wlr = WlrLayerShellState::new::<I>(&display_handle);
    return Layershell {
        wlr
    }
}
