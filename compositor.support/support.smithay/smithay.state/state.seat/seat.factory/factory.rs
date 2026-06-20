use smithay::input::pointer::CursorImageStatus;
use smithay::input::{Seat, SeatHandler, SeatState};
use smithay::reexports::wayland_protocols::wp::pointer_constraints::zv1::server::zwp_confined_pointer_v1::ZwpConfinedPointerV1;
use smithay::reexports::wayland_protocols::wp::pointer_constraints::zv1::server::zwp_locked_pointer_v1::ZwpLockedPointerV1;
use smithay::reexports::wayland_protocols::wp::pointer_constraints::zv1::server::zwp_pointer_constraints_v1::ZwpPointerConstraintsV1;
use smithay::reexports::wayland_protocols::wp::relative_pointer::zv1::server::zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1;
use smithay::reexports::wayland_protocols::wp::relative_pointer::zv1::server::zwp_relative_pointer_v1::ZwpRelativePointerV1;
use smithay::reexports::wayland_server::protocol::wl_seat::WlSeat;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::pointer_constraints::{PointerConstraintsState, PointerConstraintUserData};
use smithay::wayland::relative_pointer::{RelativePointerManagerState, RelativePointerUserData};
use smithay::wayland::seat::{SeatGlobalData, WaylandFocus};
use compositor_support_smithay_dispatch_state_base::state::DispatchWire;

pub fn new<I: DispatchWire>(
    display_handle: &DisplayHandle,
) -> compositor_support_smithay_state_seat_base::state::Seat<I>
where
    I: GlobalDispatch<ZwpRelativePointerManagerV1, GlobalData>,
    I: Dispatch<ZwpRelativePointerManagerV1, GlobalData>,
    I: Dispatch<ZwpRelativePointerV1, RelativePointerUserData<I>>,
    I: GlobalDispatch<WlSeat, SeatGlobalData<I>> + SeatHandler + 'static,
    I: GlobalDispatch<ZwpPointerConstraintsV1, GlobalData>,
    I: Dispatch<ZwpPointerConstraintsV1, GlobalData>,
    I: Dispatch<ZwpConfinedPointerV1, PointerConstraintUserData<I>>,
    I: Dispatch<ZwpLockedPointerV1, PointerConstraintUserData<I>>,
    <I as SeatHandler>::PointerFocus: WaylandFocus,
    <I as SeatHandler>::KeyboardFocus: WaylandFocus,
{
    // A seat is a group of keyboards, pointer, and touch devices.
    // It maintains keyboard focus (who gets keystrokes) and pointer focus (who gets mouse events).
    let mut seat_state = SeatState::new();

    // Creates the actual `wl_seat` global named "winit" (often renamed to "seat0" in real setups).
    let mut seat: Seat<I> = seat_state.new_wl_seat(&display_handle, "winit");

    // Notify clients that we have a keyboard.
    // **Calloop Interaction:** Later in your code, you will read raw key events from your backend
    // (like libinput or Winit) during a `calloop` tick. You will feed those events into
    // `seat.get_keyboard().unwrap().input(...)`. That method translates the raw scancodes into
    // Wayland events and flushes them to the currently focused client.
    //
    // The parameters here set the repeat rate (200ms delay, 25 repeats/sec).
    seat.add_keyboard(Default::default(), 200, 25).unwrap();

    // Notify clients that we have a pointer (mouse).
    // Side-effect: Clients use this to render their own cursor icons. When you dispatch mouse
    // motion events through the seat, it triggers `enter`, `leave`, and `motion` events on the
    // client, causing them to draw hover states (like highlighting a button).
    seat.add_pointer();

    let relative_pointer_manager_state = RelativePointerManagerState::new::<I>(&display_handle);

    let pointer_constraints_state = PointerConstraintsState::new::<I>(&display_handle);

    return compositor_support_smithay_state_seat_base::state::Seat {
        state: seat_state,
        seat: seat,
        pointer_status: CursorImageStatus::default_named(),
        relative_pointer_manager_state,
        pointer_constraints_state,
        unlock_restoration_location: None,
        previous_focus: None,
        libseat: None,
    };
}
