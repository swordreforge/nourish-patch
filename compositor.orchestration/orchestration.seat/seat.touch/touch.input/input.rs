use smithay::backend::input::{
    AbsolutePositionEvent, Event, InputBackend, TouchCancelEvent, TouchDownEvent,
    TouchEvent, TouchFrameEvent, TouchMotionEvent, TouchUpEvent,
};
use smithay::utils::{Logical, Physical, Point};
use compositor_orchestration_core_state_base::state::CoordinateTrait;
use compositor_orchestration_core_state_base::{Loop, Transform};
use compositor_orchestration_seat_touch_state::state::TouchState;
use compositor_support_system_input_event_base::base::{InputEvent, InputFlow, TouchPhase};

/// Handle a touch-down event: record the touch point, map to world space,
/// and route through the input bus.
pub fn down<I: InputBackend>(
    event: &<I as InputBackend>::TouchDownEvent,
    _loop: &mut Loop,
) {
    // Map the absolute position through the compositor coordinate pipeline:
    // screen-physical → world-storage space (same as absolute pointer motion).
    let screen = _loop.size_ctx_all();
    let physical_size_as_logical = smithay::utils::Size::<i32, Logical>::from((
        screen.screen_size_physical.0.round() as i32,
        screen.screen_size_physical.1.round() as i32,
    ));
    let raw_pos: Point<f64, Logical> = event.position_transformed(physical_size_as_logical);
    let position_screen = Point::<f64, Physical>::from((raw_pos.x, raw_pos.y));

    let ctx = _loop.pointer_context(position_screen);
    let t: Transform = (position_screen, ctx).into();
    let position_normalized = t.into_storage_point_f64();

    let slot = i32::from(event.slot());

    // Update TouchState.
    let pos = Point::<f64, Logical>::from((position_normalized.x, position_normalized.y));
    _loop.inner.touch.down(slot, pos);

    // Route to the input bus.
    let ev = InputEvent::Touch {
        phase: TouchPhase::Down,
        slot,
        x: position_normalized.x,
        y: position_normalized.y,
    };
    compositor_orchestration_input_drive_base::drive::route(_loop, ev);
}

/// Handle a touch-motion event: update position, map to world space,
/// and route through the input bus.
pub fn motion<I: InputBackend>(
    event: &<I as InputBackend>::TouchMotionEvent,
    _loop: &mut Loop,
) {
    let screen = _loop.size_ctx_all();
    let physical_size_as_logical = smithay::utils::Size::<i32, Logical>::from((
        screen.screen_size_physical.0.round() as i32,
        screen.screen_size_physical.1.round() as i32,
    ));
    let raw_pos: Point<f64, Logical> = event.position_transformed(physical_size_as_logical);
    let position_screen = Point::<f64, Physical>::from((raw_pos.x, raw_pos.y));

    let ctx = _loop.pointer_context(position_screen);
    let t: Transform = (position_screen, ctx).into();
    let position_normalized = t.into_storage_point_f64();

    let slot = i32::from(event.slot());

    // Update TouchState.
    let pos = Point::<f64, Logical>::from((position_normalized.x, position_normalized.y));
    _loop.inner.touch.motion(slot, pos);

    // Route to the input bus.
    let ev = InputEvent::Touch {
        phase: TouchPhase::Motion,
        slot,
        x: position_normalized.x,
        y: position_normalized.y,
    };
    compositor_orchestration_input_drive_base::drive::route(_loop, ev);
}

/// Handle a touch-up event: remove the touch point and route through the input bus.
pub fn up<I: InputBackend>(
    event: &<I as InputBackend>::TouchUpEvent,
    _loop: &mut Loop,
) {
    let slot = i32::from(event.slot());

    // Update TouchState.
    _loop.inner.touch.up(slot);

    // Route to the input bus. TouchUp has no position data.
    let ev = InputEvent::Touch {
        phase: TouchPhase::Up,
        slot,
        x: 0.0,
        y: 0.0,
    };
    compositor_orchestration_input_drive_base::drive::route(_loop, ev);
}

/// Handle a touch-cancel event: clear all touch points and route through the input bus.
pub fn cancel<I: InputBackend>(
    _event: &<I as InputBackend>::TouchCancelEvent,
    _loop: &mut Loop,
) {
    // Update TouchState.
    _loop.inner.touch.cancel();

    // Route to the input bus.
    let ev = InputEvent::Touch {
        phase: TouchPhase::Cancel,
        slot: -1,
        x: 0.0,
        y: 0.0,
    };
    compositor_orchestration_input_drive_base::drive::route(_loop, ev);
}

/// Handle a touch-frame event: no bus event needed (the frame just signals
/// that the preceding events form an atomic batch).
pub fn frame<I: InputBackend>(
    _event: &<I as InputBackend>::TouchFrameEvent,
    _loop: &mut Loop,
) {
    // Nothing to update or route — the bus receivers already processed each
    // individual down/motion/up/cancel event synchronously. The frame boundary
    // is meaningful only for Wayland protocol forwarding (future P2+).
    info!("TouchFrame received");
}
