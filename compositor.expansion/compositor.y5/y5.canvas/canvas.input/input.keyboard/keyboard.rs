use crate::{input, navigator};
use smithay::backend::input::KeyState;
use smithay::input::keyboard::{Keysym, ModifiersState, keysyms};
use compositor_orchestration_core_state_base::Loop;
use compositor_support_library_input_keyboard_base::keyboard::key::Key;

pub fn input_received(
    key_state: KeyState,
    modifiers: &ModifiersState,
    keysym: Keysym, // D = Dispatch flip: the caller extracts the keysym from the
                    // smithay seat callback (which only sees `&mut Dispatch`) so
                    // this runs against the full `&mut Loop`.
    state: &mut Loop,
) -> Option<bool> {
    let key = Key::from_keysym(keysym);
    // println!("Handling key: {:?} {:?} {:?}", key_state, modifiers, keysym);

    input::input_received(key_state, modifiers, state, key)?;
    navigator::input_received(key_state, modifiers, state, key)?;

    Some(true)
}
