use smithay::backend::input::KeyState;
use smithay::input::keyboard::{KeysymHandle, ModifiersState};
use compositor_orchestration_core_state_base::Loop;

pub fn keyboard_received(
    key_state: KeyState,
    modifiers: &ModifiersState,
    state: &mut Loop, // unified variable name to fix compilation
) -> Option<bool> {
    // If launcher is active, send it all events and intercept.
    let Some(handle) = state.inner.launcher_mut().handle else {
        return Some(true);
    };

    let Some(ref mut registry) = state.inner.surface_mut().registry else {
        return Some(true);
    };

    registry.set_keyboard_focus(Some(handle.id));

    None
}
