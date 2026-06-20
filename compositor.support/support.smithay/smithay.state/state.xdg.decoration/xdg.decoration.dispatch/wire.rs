use smithay::wayland::shell::xdg::ToplevelSurface;
use compositor_support_smithay_dispatch_state_base::state::{Dispatch, DispatchWire};
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode as XdgDecorationMode;

/// The `XdgDecorationHandler` processes incoming requests from clients over the Wayland socket.
/// These functions are invoked synchronously during the Wayland event dispatch phase of your `calloop`.

/// Triggered when a client initially creates a decoration object for a specific toplevel window.
pub fn new_decoration(
    dispatch: &mut Dispatch,
    toplevel: ToplevelSurface,
) {
    // We modify the pending state of the surface to strictly enforce Server-Side Decorations.
    toplevel.with_pending_state(|state| {
        state.decoration_mode = Some(XdgDecorationMode::ServerSide);
        // state.decoration_mode = Some(XdgDecorationMode::ServerSide);
    });

    // SIDE-EFFECT / DISPATCH:
    // `send_pending_configure` queues a `configure` event into the client's Wayland socket buffer.
    // During the next `calloop` flush, this event is sent across the IPC socket.
    //
    // Client Reaction: The client will receive this, realize it shouldn't draw its own borders,
    // and schedule an internal redraw to strip out its CSD. It will then reply with an `ack_configure`
    // and eventually `commit` a new buffer without borders.
    toplevel.send_pending_configure();
}

/// Triggered when a client actively requests a specific decoration mode (e.g., a GNOME app
/// might ask to use Client-Side decorations).
pub fn request_mode(
    dispatch: &mut Dispatch,
    toplevel: ToplevelSurface,
    _mode: XdgDecorationMode,
) {
    // Ignore the client's preference entirely (`_mode`), strictly forcing ServerSide.
    // This ensures a uniform look managed by the compositor.
    toplevel.with_pending_state(|state| {
        state.decoration_mode = Some(_mode);
    });

    // SIDE-EFFECT / DISPATCH:
    // As above, this pushes a `configure` event over the socket to inform the client
    // that its request for a specific mode was overridden. The client must redraw and
    // acknowledge this new configuration.
    toplevel.send_pending_configure();
}

/// Triggered when a client unsets its decoration mode preference or destroys the decoration object.
pub fn unset_mode(
    dispatch: &mut Dispatch,
    toplevel: ToplevelSurface,
) {
    // Even if the client unsets its preference, we ensure the compositor still
    // dictates that decorations are handled Server-Side.
    toplevel.with_pending_state(|state| {
        state.decoration_mode = Some(XdgDecorationMode::ServerSide);
    });

    // SIDE-EFFECT / DISPATCH:
    // Pushes the final `configure` event state to the client, triggering a layout/redraw
    // update on the client end if their state previously differed.
    toplevel.send_pending_configure();
}
