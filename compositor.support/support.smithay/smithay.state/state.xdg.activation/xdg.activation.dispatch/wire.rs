use smithay::wayland::xdg_activation::XdgActivationState;
use compositor_support_smithay_dispatch_state_base::state::{Dispatch, DispatchWire};

pub use compositor_support_smithay_state_xdg_activation_request::{ActivationDetails, request_activation};

pub fn activation_state(
    dispatch: &mut Dispatch,
) -> &mut XdgActivationState {
    &mut dispatch.xdg_activation.xdg_activation
}
