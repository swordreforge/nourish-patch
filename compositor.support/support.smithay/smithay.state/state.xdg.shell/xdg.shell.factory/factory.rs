use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_wm_base::XdgWmBase;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::shell::xdg::XdgShellState;
use compositor_support_smithay_dispatch_state_base::state::DispatchWire;
use compositor_support_smithay_state_xdg_shell_base::state::XDGShell;

pub fn new<I: DispatchWire>(display_handle: &DisplayHandle) -> XDGShell where
    I: GlobalDispatch<XdgWmBase, GlobalData> + 'static {
    // Initialize the XDG Shell protocol.
    // Side-effect: Triggers window mapping/unmapping. When a client requests a new Toplevel,
    // it dispatches an event in your XDG delegate to assign the window a position in the `Space`.
    let xdg_shell_state = XdgShellState::new::<I>(&display_handle);
    XDGShell {
        state: xdg_shell_state
    }
}
