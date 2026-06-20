use smithay::input::{Seat, SeatHandler, SeatState};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point};
use smithay::wayland::seat::WaylandFocus;
use compositor_support_smithay_dispatch_state_base::state::{Dispatch, DispatchWire};

pub use compositor_support_smithay_state_seat_focus::RestorationToken as restoration_token;
// `focus_changed` moved inline into `impl SeatHandler for Dispatch` (state.base);
// the standalone fn no longer exists (document/SMITHAY_DECOUPLING.md).

/// `SeatHandler` manages the state of the user's physical presence (keyboard, mouse, touch).
///
/// **Compositor Hookup:**
/// When you read raw input events (e.g., from `libinput` via `calloop`), you feed them into
/// the `Seat`. The `Seat` then determines which `WlSurface` currently has focus and dispatches
/// the Wayland protocol events to that specific client.

// We define that focus is tracked at the granularity of a Wayland Surface.

pub fn seat_state(
    dispatch: &mut Dispatch,
) -> &mut SeatState<Dispatch> {
    return &mut dispatch.seat.state;
}

/// Triggered when a client wants to change the mouse cursor icon (e.g., to a text cursor
/// or a resize arrow).
///
/// **Side-effect / Redraw:** In a full implementation, you would save `_image` to your state
/// and use it to draw the cursor texture on top of your windows during your render loop.
pub fn cursor_image(
    dispatch: &mut Dispatch,
    _seat: &Seat<Dispatch>,
    _image: smithay::input::pointer::CursorImageStatus,
) {
    dispatch.seat.pointer_status = _image;
}

// pub fn get_warp_pointer_to_surface(
//     dispatch: &mut Dispatch,
//     clock_ms: u32,
//     surface: &WlSurface,
//     surface_local: Point<f64, Logical>,
//     global_location: Point<f64, Logical>,
// ) -> Option<(WlSurface, Point<f64, Logical>, Point<f64, Logical>)> {
//     // let Some(pointer) = dispatch.seat.seat.get_pointer() else {
//     //     return None;
//     // };
//     // pointer.motion(
//     //     self,
//     //     Some((surface.clone(), surface_local)),
//     //     &MotionEvent {
//     //         location: global_location,
//     //         serial,
//     //         time,
//     //     },
//     // );
//     // pointer.frame(self);

//     // let serial = SERIAL_COUNTER.next_serial();
//     // let time = self.clock.now().as_millis() as u32;

//     return Some((surface.clone(), surface_local, global_location));
// }
