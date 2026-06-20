use smithay::reexports::wayland_protocols::xdg::activation::v1::server::xdg_activation_v1;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::xdg_activation::{XdgActivationHandler, XdgActivationState};
use compositor_support_smithay_dispatch_state_base::state::DispatchWire;
use compositor_support_smithay_state_xdg_activation_base::state::Activation;

pub fn new<I: DispatchWire>(display_handle: &DisplayHandle) -> Activation
where
    I: GlobalDispatch<xdg_activation_v1::XdgActivationV1, GlobalData>
        + Dispatch<xdg_activation_v1::XdgActivationV1, GlobalData>
        + XdgActivationHandler
        + 'static,
{
    let xdg_activation_state = XdgActivationState::new::<I>(&display_handle);

    Activation {
        xdg_activation: xdg_activation_state,
    }
}
