use smithay::wayland::shell::xdg::XdgShellState;
use compositor_support_smithay_dispatch_state_base::state::{Dispatch, DispatchWire};

pub fn xdg_shell_state(
    dispatch: &mut Dispatch,
) -> &mut XdgShellState {
    &mut dispatch.xdg_shell.state
}

// `new_toplevel` / `destroy_toplevel` were removed: toplevel create/destroy now
// records into the Dispatch outbox (handler) and applies at drain
// (document/SMITHAY_DECOUPLING.md), so support.smithay no longer touches the
// world here.
