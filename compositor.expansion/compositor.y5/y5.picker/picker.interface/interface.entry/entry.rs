use compositor_orchestration_core_state_base::state::SetPickerRequest;
use compositor_orchestration_core_state_base::Loop;
use compositor_y5_picker_system_base::base::{PICKER_MUT, PICKER_WORLD};

/// SUPER+K behaviour: request the picker if it isn't showing, cancel it if it
/// is. Opening is deferred (a renderer is needed to snapshot the current world
/// for its thumbnail — see `interface.capture`); cancelling is immediate.
pub fn toggle(state: &mut Loop) {
    if state.inner.worlds.active_id() == PICKER_WORLD {
        compositor_y5_picker_interface_base::base::cancel(state);
    } else {
        request_open(state);
    }
}

/// Flag the deferred open request. The GLES prepare phase drains it to arm +
/// snapshot the origin world before switching. No-op if a request/arm is
/// already in flight, so repeated presses don't queue multiple captures.
pub fn request_open(state: &mut Loop) {
    if state.inner.worlds.active_id() == PICKER_WORLD {
        return;
    }
    let arming = state
        .inner
        .worlds
        .get_mut(PICKER_WORLD)
        .storage_mut()
        .get_mut(&PICKER_MUT)
        .arming
        .is_some();
    if state.inner.__set_picker.is_some() || arming {
        return;
    }
    state.inner.__set_picker = Some(SetPickerRequest::Open);
}
