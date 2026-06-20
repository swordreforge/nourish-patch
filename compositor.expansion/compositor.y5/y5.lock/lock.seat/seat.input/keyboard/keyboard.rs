use crate::keyboard::iced;
use smithay::backend::input::{
    Axis, AxisSource, Event, InputBackend, KeyState, KeyboardKeyEvent, PointerAxisEvent,
};
use smithay::input::keyboard::{FilterResult, Keysym, ModifiersState};
use smithay::input::pointer::{AxisFrame, PointerHandle};
use smithay::utils::SERIAL_COUNTER;
use compositor_orchestration_core_state_base::Loop;
use compositor_support_smithay_dispatch_state_base::state::Dispatch;

pub fn input_received<I: InputBackend>(event: &I::KeyboardKeyEvent, _loop: &mut Loop) {
    let serial = SERIAL_COUNTER.next_serial();
    let time = Event::time_msec(event);
    let key_state = event.state();
    let key_code = event.key_code();

    // The smithay seat callback only sees `&mut Dispatch` now (D = Dispatch,
    // document/SMITHAY_DECOUPLING.md), but the lock-screen shortcut handlers need
    // the whole Loop (world + loop_handle). So the callback only EXTRACTS the
    // modified keysym + modifiers (Copy) and always intercepts at the wayland
    // level; the world-touching handlers run AFTER, with `_loop` free again.
    let mut extracted: Option<(Keysym, ModifiersState)> = None;
    _loop
        .state
        .seat
        .seat
        .get_keyboard()
        .unwrap()
        .input::<(), _>(
            &mut _loop.state,
            key_code,
            key_state,
            serial,
            time,
            |_state: &mut Dispatch, modifiers, handle| {
                extracted = Some((handle.modified_sym().clone(), *modifiers));
                // No visible windows on the lock screen — always intercept.
                FilterResult::Intercept(())
            },
        );

    if let Some((keysym, modifiers)) = extracted {
        // Overlay shortcuts first, then the iced lock surface.
        let intercepted = compositor_y5_overlay_interface_keyboard::keyboard::input_received::<I>(
            _loop, keysym, key_state, &modifiers,
        );
        if intercepted {
            return;
        }
        let _ = iced::input_received::<I>(_loop, keysym, key_state, &modifiers);
    }
}
