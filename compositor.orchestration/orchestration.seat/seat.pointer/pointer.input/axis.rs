use crate::native_axis;
use smithay::backend::input::{InputBackend, PointerAxisEvent as _};
use compositor_orchestration_core_state_base::Loop;

pub fn axis<I: InputBackend>(event: &<I as InputBackend>::PointerAxisEvent, _loop: &mut Loop) {
    {
        let location = _loop.state.seat.seat.get_pointer().unwrap().current_location();
        let h = smithay::backend::input::Axis::Horizontal;
        let v = smithay::backend::input::Axis::Vertical;
        let ev = compositor_support_system_input_event_base::base::InputEvent::PointerAxis {
            horizontal: event.amount(h).unwrap_or_else(|| event.amount_v120(h).unwrap_or(0.0)),
            vertical: event.amount(v).unwrap_or_else(|| event.amount_v120(v).unwrap_or(0.0)),
            x: location.x,
            y: location.y,
        };
        if compositor_orchestration_input_drive_base::drive::route(_loop, ev)
            == compositor_support_system_input_event_base::base::InputFlow::Consume
        {
            return;
        }
    }
    // Pass from the bus means the cursor is over a window and not a hand-pan
    // (CameraSystem::input consumes the canvas-zoom case), so it's a window scroll.
    let pointer = _loop.state.seat.seat.get_pointer().unwrap();
    native_axis::axis::input_received::<I>(pointer, event, _loop);
}
