//
// Drag and Drop (DnD)
//
// impl DndGrabHandler for Loop {}

/// Handles the start of a Drag-and-Drop operation.
///
/// **What is happening here?**
/// When a user clicks and drags a file or highlighted text in an app, that app sends a
/// `start_drag` request over the socket. `calloop` wakes up, routes the request to Smithay,
/// and Smithay calls this function.
pub mod WaylandDndGrabHandler {
    use smithay::input::Seat;
    use smithay::input::dnd::{DnDGrab, GrabType, Source};
    use smithay::input::pointer::{Focus, PointerHandle};
    use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
    use smithay::utils::Serial;
    use compositor_support_smithay_dispatch_state_base::state::{Dispatch, DispatchWire};

    pub fn dnd_requested_prepare<S: Source, WireObject: DispatchWire>(
        dispatch: &mut Dispatch,
        source: S,
        // The app might provide a custom icon to float under the mouse
        _icon: Option<WlSurface>,
        seat: Seat<WireObject>,
        serial: Serial,
        type_: GrabType,
    ) -> Option<(
        PointerHandle<WireObject>,
        DnDGrab<WireObject, S, WlSurface>,
        Serial,
        Focus,
    )> {
        match type_ {
            GrabType::Pointer => {
                let ptr = seat.get_pointer().unwrap();
                let start_data = ptr.grab_start_data().unwrap();

                // SIDE-EFFECT / WAYLAND DISPATCH:
                // We create a `DnDGrab`. This works exactly like the `GrabMovement` we saw
                // earlier. It intercepts all mouse movement. Instead of moving a window,
                // the `DnDGrab` calculates which window the mouse is hovering over, and sends
                // `wl_data_device.enter`, `motion`, and `leave` events to the clients beneath
                // the cursor so they can highlight themselves if they accept dropped data.
                let grab =
                    DnDGrab::new_pointer(&dispatch.output.display_handle, start_data, source, seat);
                dispatch.dnd.icon = _icon;
                return Some((ptr, grab, serial, Focus::Keep));
            }
            GrabType::Touch => {
                // Since this compositor lacks touch handling, we explicitly cancel the source.
                // This dispatches a message to the client over the socket saying the drag failed.
                source.cancel();
            }
        }

        return None;
    }

    pub fn dnd_requested_bind<S: Source, WireObject: DispatchWire>(
        wire: &mut WireObject,
        result: (
            PointerHandle<WireObject>,
            DnDGrab<WireObject, S, WlSurface>,
            Serial,
            Focus,
        ),
    ) {
        let (pointer, grab, serial, focus) = result;
        // Focus::Keep means we don't drop keyboard focus from the original app just
        // because the user is dragging something out of it.
        pointer.set_grab(wire, grab, serial, Focus::Keep);
    }
}
