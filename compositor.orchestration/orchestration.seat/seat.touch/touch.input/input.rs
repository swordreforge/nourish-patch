use smithay::backend::input::{
    AbsolutePositionEvent, Event, InputBackend, TouchCancelEvent, TouchDownEvent,
    TouchEvent, TouchFrameEvent, TouchMotionEvent, TouchUpEvent,
};
use smithay::input::touch::{DownEvent, MotionEvent, UpEvent};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Physical, Point, SERIAL_COUNTER};
use compositor_orchestration_core_state_base::state::CoordinateTrait;
use compositor_orchestration_core_state_base::{Loop, Transform};
use compositor_orchestration_seat_touch_state::state::TouchState;
use compositor_support_system_input_event_base::base::{InputEvent, InputFlow, TouchPhase};
use compositor_y5_surface_interface_base::hit::surface_under_filtered;

/// Map a touch event's absolute position through the compositor coordinate pipeline:
/// screen-physical → world-storage space.
fn touch_position<I: InputBackend>(
    event: &impl AbsolutePositionEvent<I>,
    _loop: &mut Loop,
) -> Point<f64, Logical> {
    let screen = _loop.size_ctx_all();
    let physical_size_as_logical = smithay::utils::Size::<i32, Logical>::from((
        screen.screen_size_physical.0.round() as i32,
        screen.screen_size_physical.1.round() as i32,
    ));
    let raw_pos: Point<f64, Logical> = event.position_transformed(physical_size_as_logical);
    let position_screen = Point::<f64, Physical>::from((raw_pos.x, raw_pos.y));
    let ctx = _loop.pointer_context(position_screen);
    let t: Transform = (position_screen, ctx).into();
    t.into_storage_point_f64()
}

/// Find the surface and surface-local position under a touch point.
fn surface_focus(
    _loop: &Loop,
    pos: Point<f64, Logical>,
) -> Option<(WlSurface, Point<f64, Logical>)> {
    let hit = surface_under_filtered(_loop, pos, &|_hit| true)?;
    let surface = hit.surface()?.clone();
    let position = hit.position_motion()?;
    Some((surface, position))
}

/// Handle a touch-down event: record the touch point, map to world space,
/// route through the input bus, and forward to the Wayland client.
pub fn down<I: InputBackend>(
    event: &<I as InputBackend>::TouchDownEvent,
    _loop: &mut Loop,
) {
    let pos = touch_position(event, _loop);
    let touch_slot = event.slot();
    let slot = i32::from(touch_slot);
    let serial = SERIAL_COUNTER.next_serial();
    let time = event.time_msec();

    // Update TouchState.
    _loop.inner.touch.down(slot, pos);

    // Route to the input bus.
    let ev = InputEvent::Touch {
        phase: TouchPhase::Down,
        slot,
        x: pos.x,
        y: pos.y,
    };
    if compositor_orchestration_input_drive_base::drive::route(_loop, ev)
        == InputFlow::Consume
    {
        return;
    }

    // Forward to the Wayland client via TouchHandle.
    if let Some(handle) = _loop.state.seat.seat.get_touch() {
        let focus = surface_focus(_loop, pos);
        handle.down(
            &mut _loop.state,
            focus,
            &DownEvent {
                slot: touch_slot,
                location: pos,
                serial,
                time,
            },
        );
    }
}

/// Handle a touch-motion event: update position, map to world space,
/// route through the input bus, and forward to the Wayland client.
pub fn motion<I: InputBackend>(
    event: &<I as InputBackend>::TouchMotionEvent,
    _loop: &mut Loop,
) {
    let pos = touch_position(event, _loop);
    let touch_slot = event.slot();
    let slot = i32::from(touch_slot);
    let time = event.time_msec();

    // Update TouchState.
    _loop.inner.touch.motion(slot, pos);

    // Route to the input bus.
    let ev = InputEvent::Touch {
        phase: TouchPhase::Motion,
        slot,
        x: pos.x,
        y: pos.y,
    };
    if compositor_orchestration_input_drive_base::drive::route(_loop, ev)
        == InputFlow::Consume
    {
        return;
    }

    // Forward to the Wayland client via TouchHandle.
    if let Some(handle) = _loop.state.seat.seat.get_touch() {
        let focus = surface_focus(_loop, pos);
        handle.motion(
            &mut _loop.state,
            focus,
            &MotionEvent {
                slot: touch_slot,
                location: pos,
                time,
            },
        );
    }
}

/// Handle a touch-up event: remove the touch point, route through the input bus,
/// and forward to the Wayland client.
pub fn up<I: InputBackend>(
    event: &<I as InputBackend>::TouchUpEvent,
    _loop: &mut Loop,
) {
    let touch_slot = event.slot();
    let slot = i32::from(touch_slot);
    let serial = SERIAL_COUNTER.next_serial();
    let time = event.time_msec();

    // Update TouchState.
    _loop.inner.touch.up(slot);

    // Route to the input bus. TouchUp has no position data.
    let ev = InputEvent::Touch {
        phase: TouchPhase::Up,
        slot,
        x: 0.0,
        y: 0.0,
    };
    if compositor_orchestration_input_drive_base::drive::route(_loop, ev)
        == InputFlow::Consume
    {
        return;
    }

    // Forward to the Wayland client via TouchHandle.
    if let Some(handle) = _loop.state.seat.seat.get_touch() {
        handle.up(
            &mut _loop.state,
            &UpEvent {
                slot: touch_slot,
                serial,
                time,
            },
        );
    }
}

/// Handle a touch-cancel event: clear all touch points, route through the input bus,
/// and forward to the Wayland client.
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

    // Forward to the Wayland client via TouchHandle.
    if let Some(handle) = _loop.state.seat.seat.get_touch() {
        handle.cancel(&mut _loop.state);
    }
}

/// Handle a touch-frame event: no bus event needed (the frame just signals
/// that the preceding events form an atomic batch), but forward the frame
/// marker to the Wayland client.
pub fn frame<I: InputBackend>(
    _event: &<I as InputBackend>::TouchFrameEvent,
    _loop: &mut Loop,
) {
    // Forward the frame marker to the Wayland client via TouchHandle.
    if let Some(handle) = _loop.state.seat.seat.get_touch() {
        handle.frame(&mut _loop.state);
    }
}
