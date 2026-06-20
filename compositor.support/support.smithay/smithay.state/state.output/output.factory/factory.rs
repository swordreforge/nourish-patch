use smithay::reexports::wayland_protocols::xdg::xdg_output::zv1::server::zxdg_output_manager_v1::ZxdgOutputManagerV1;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::output::{OutputManagerState, WlOutputData};
use compositor_support_smithay_dispatch_state_base::state::DispatchWire;
use compositor_support_smithay_state_output_base::state::OutputState;

pub fn new<I: DispatchWire>(display_handle: &DisplayHandle) -> OutputState
where
    I: GlobalDispatch<WlOutput, WlOutputData>,
    I: GlobalDispatch<ZxdgOutputManagerV1, GlobalData>,
    I: 'static,
{
    // Initialize Output management.
    // Side-effect: When you add an output (monitor) to this state later, it immediately pushes
    // events over the socket telling clients about the new screen size, triggering clients to
    // calculate layout and redraw.
    let output_manager_state = OutputManagerState::new_with_xdg_output::<I>(&display_handle);

    return OutputState {
        display_handle: display_handle.clone(),
        output_manager_state,
    };
}
