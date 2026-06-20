use std::sync::atomic::Ordering;
use smithay_client_toolkit::shell::WaylandSurface;
use wayland_client::{
    protocol::{wl_callback, wl_surface},
    Connection, Dispatch, QueueHandle,
};

use crate::state::OverlayClient;



pub fn pump(state: &mut OverlayClient, qh: &QueueHandle<OverlayClient>) {
    let event_dirty = state.iced.as_mut().map(|d| d.update()).unwrap_or(false);
    let needs_first_draw = !state.drawn_initial_frame && state.iced.is_some();
    let shell_dirty = state.redraw_requested.swap(false, Ordering::Relaxed);
    let dirty = event_dirty || shell_dirty;

    // tracing::info!(
    //     "pump: event_dirty={}, frame_in_flight={}, callback_fired={}",
    //     event_dirty, state.frame_in_flight, state.frame_callback_fired
    // );
    // tracing::info!(
    //     "pump: event={}, needs_first_draw={}, shell_dirty={} , dirty={}",
    //     event_dirty, needs_first_draw, shell_dirty, dirty
    // );

    if state.layout_invalidated.swap(false, Ordering::Relaxed) {
        if let Some(iced) = state.iced.as_mut() {
            iced.invalidate_layout();
        }
    }

    // First frame: render eagerly, no frame callback dance.
    if needs_first_draw {
        if let (Some(layer), Some(iced)) = (&state.layer, state.iced.as_mut()) {
            iced.render_frame();
            // Request a callback for subsequent frames.
            let surface = layer.wl_surface();
            surface.frame(qh, surface.clone());
            state.frame_in_flight = false;
            state.drawn_initial_frame = true;
        }
        return;
    }

    // Subsequent frames: callback-driven.
    if dirty && !state.frame_in_flight {
        if let Some(layer) = &state.layer {
            let surface = layer.wl_surface();
            surface.frame(qh, surface.clone());
            state.frame_in_flight = true;
            surface.commit();
        }
    }

    if state.frame_callback_fired {
        state.frame_callback_fired = false;
        state.frame_in_flight = false;
        if let Some(iced) = state.iced.as_mut() {
            iced.render_frame();
        }
        state.drawn_initial_frame = true;

    }
}