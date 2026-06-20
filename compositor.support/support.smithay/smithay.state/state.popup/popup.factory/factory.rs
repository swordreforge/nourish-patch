use smithay::desktop::PopupManager;
use compositor_support_smithay_dispatch_state_base::state::DispatchWire;
use compositor_support_smithay_state_popup_base::state::PopupState;

pub fn new<I: DispatchWire>() -> PopupState {
    let popup = PopupManager::default();

    return PopupState { state: popup };
}
