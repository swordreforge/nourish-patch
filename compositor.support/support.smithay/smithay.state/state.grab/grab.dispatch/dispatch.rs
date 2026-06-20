use smithay::backend::renderer::element::Element;
use smithay::input::pointer::GrabStartData;
use smithay::input::{Seat, SeatHandler};
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::Serial;
use compositor_support_smithay_dispatch_state_base::state::DispatchWire;

/// Validates that a client requesting a privileged action (like moving or resizing a window)
/// is actually permitted to do so.
pub fn check_grab<WireObject: DispatchWire>(
    seat: &Seat<WireObject>,
    surface: &WlSurface,
    serial: Serial,
) -> Option<GrabStartData<WireObject>>
// where
//     WireObject: SeatHandler<PointerFocus = WlSurface> + 'static,
{
    let pointer = seat.get_pointer()?;

    // Check that this surface has a click grab.

    // `Serial` is Wayland's mechanism for preventing race conditions.
    // Every input event gets a unique serial number. We check if the serial provided by the
    // client matches the *actual* serial of the latest mouse click known to the compositor.
    if !pointer.has_grab(serial) {
        return None;
    }

    let start_data = pointer.grab_start_data()?;

    let (focus, _) = start_data.focus.as_ref()?;

    // If the focus was for a different surface, ignore the request.

    // Prevent malicious clients from asking to move *other* clients' windows.
    // The surface that currently has the physical mouse focus must belong to the same
    // Wayland client that is sending the move/resize request over the socket.
    if !focus.id().same_client_as(&surface.id()) {
        return None;
    }

    Some(start_data)
}
