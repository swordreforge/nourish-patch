//
// Wl Data Device (Clipboard)
//

/// Boilerplate handler required by Smithay to track internal clipboard selection state.
pub mod SelectionHandler {
    pub type SelectionUserData = ();
}

/// Gives Smithay access to your `DataDeviceState` so it can handle clipboard copy/paste
/// requests internally when clients talk to the `wl_data_device` protocol.
pub mod DataDeviceHandler {
    use smithay::wayland::selection::data_device::DataDeviceState;
    use compositor_support_smithay_dispatch_state_base::state::{Dispatch, DispatchWire};

    pub fn data_device_state(dispatch: &mut Dispatch) -> &mut DataDeviceState {
        &mut dispatch.clipboard.data_device_state
    }
}
