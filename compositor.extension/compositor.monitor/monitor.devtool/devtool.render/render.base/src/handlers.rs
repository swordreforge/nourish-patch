use smithay_client_toolkit::{
    compositor::CompositorHandler,
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output,
    delegate_pointer, delegate_registry, delegate_seat,
    output::{OutputHandler, OutputState},
    registry::ProvidesRegistryState,
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
        pointer::{PointerEvent, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
        WaylandSurface,
    },
};
use wayland_client::{
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_surface},
    Connection, Dispatch, QueueHandle,
};
use compositor_monitor_devtool_scene_base::app::CompositorSnapshot;
use compositor_monitor_server_protocol_base::protocol::y5_proto::y5_compositor_unstable_client_v1::y5_compositor_manager_v1::{
    self, Y5CompositorManagerV1,
};

use crate::driver::IcedDriver;
use crate::input::{translate_key, translate_modifiers, translate_pointer};
use crate::state::{OverlayClient, OverlayMessageHandler};

use iced_core::{
    Event as IcedEvent, Point,
    keyboard::{self, Key, Location, Modifiers as IcedMods, key::Named},
    mouse::{self, Button, ScrollDelta},
    window,
};
use tonic::{IntoRequest, Request};
use compositor_monitor_devtool_scene_base::ui::Message;
use compositor_remote_message_server_base::message::ServerEvent;
use compositor_remote_message_server_base::message::canvas_events::CanvasMessage;
// ─── Custom protocol: no events on this interface today ───────────────────

impl Dispatch<Y5CompositorManagerV1, ()> for OverlayClient {
    fn event(
        _state: &mut Self,
        _proxy: &Y5CompositorManagerV1,
        _event: y5_compositor_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // No events defined in the protocol yet.
    }
}

// ─── Layer shell ──────────────────────────────────────────────────────────

impl LayerShellHandler for OverlayClient {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        info!("Closed received");

        self.should_exit = true;
    }

    fn configure(
        &mut self,
        conn: &Connection,
        _qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let (w, h) = configure.new_size;
        let w = if w == 0 { self.requested_size.0 } else { w };
        let h = if h == 0 { self.requested_size.1 } else { h };
        let w = if w == 0 { 800 } else { w };
        let h = if h == 0 { 600 } else { h };

        let first = self.configured_size.is_none();
        self.configured_size = Some((w, h));

        info!("Configure received");

        if first {
            info!("Creating IcedDriver");
            // Snapshot is currently empty; pass it anyway for forward compat.
            let snapshot = CompositorSnapshot::default();

            let mut iced = IcedDriver::new(
                conn,
                layer.wl_surface(),
                (w, h),
                1.0, // scale_factor — wire this to surface_scale later
                &snapshot,
                self.redraw_requested.clone(),
                self.layout_invalidated.clone(),
            );
            let handler = OverlayMessageHandler {
                grpc: self.grpc.clone(),
            };

            iced.set_message_handler(handler);

            self.iced = Some(iced);
        } else if let Some(iced) = self.iced.as_mut() {
            iced.resize((w, h), 1.0);
        }
    }
}

// ─── Compositor ───────────────────────────────────────────────────────────

impl CompositorHandler for OverlayClient {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: i32,
    ) {
    }
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }
    fn frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.frame_callback_fired = true;
    }
    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

// ─── Output ───────────────────────────────────────────────────────────────

impl OutputHandler for OverlayClient {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

// ─── Seat ─────────────────────────────────────────────────────────────────

impl SeatHandler for OverlayClient {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            self.keyboard = Some(self.seat_state.get_keyboard(qh, &seat, None).unwrap());
        }
        if capability == Capability::Pointer && self.pointer.is_none() {
            self.pointer = Some(self.seat_state.get_pointer(qh, &seat).unwrap());
        }
    }
    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _: Capability,
    ) {
    }
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

// ─── Pointer (point 1, wired) ─────────────────────────────────────────────

impl PointerHandler for OverlayClient {
    fn pointer_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        let Some(iced) = self.iced.as_mut() else {
            return;
        };
        for ev in events {
            self.pointer_position = ev.position;

            for iced_ev in translate_pointer(ev) {
                // Feed the position into the overlay's state
                iced.queue_message(Message::CursorMoved(Point::new(
                    ev.position.0 as f32,
                    ev.position.1 as f32,
                )));

                iced.queue_event(iced_ev);
            }
        }
    }
}

// ─── Keyboard (point 1, wired) ────────────────────────────────────────────

impl KeyboardHandler for OverlayClient {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
        info!("keyboard");
        if let Some(iced) = self.iced.as_mut() {
            iced.queue_event(IcedEvent::Window(window::Event::Focused));
        }
    }
    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
        info!("keyboard- leave");
        if let Some(iced) = self.iced.as_mut() {
            iced.queue_event(IcedEvent::Window(window::Event::Unfocused));
            iced.queue_message(Message::ClickedOutside);
        }
    }
    fn press_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        if let Some(iced) = self.iced.as_mut() {
            // sctk doesn't pass current modifiers into press_key; track them
            // in update_modifiers (below) and store on self if you want them
            // here. For now use empty modifiers.
            if let Some(ev) = translate_key(&event, Modifiers::default(), true, false) {
                iced.queue_event(ev);
            }

            // Also send a synthetic Message::KeyTyped for the diagnostics log.
            if let Some(text) = &event.utf8 {
                if !text.is_empty() {
                    // iced.queue_message(Message::KeyTyped(text.clone()));
                }
            }
        }
    }
    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        if let Some(iced) = self.iced.as_mut() {
            if let Some(ev) = translate_key(&event, Modifiers::default(), false, false) {
                iced.queue_event(ev);
            }
        }
    }
    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        modifiers: Modifiers,
        _: u32,
    ) {
        // self.current_modifiers = modifiers;
        if let Some(iced) = self.iced.as_mut() {
            iced.queue_message(Message::ShiftChanged(modifiers.shift));
            iced.queue_message(Message::AltChanged(modifiers.alt));
        }
        // Store modifiers on self if you want press_key to receive them.
    }
}

// ─── Registry ─────────────────────────────────────────────────────────────

impl ProvidesRegistryState for OverlayClient {
    fn registry(&mut self) -> &mut smithay_client_toolkit::registry::RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

impl OverlayClient {
    pub fn broadcast_dispatch(
        &mut self,
        message: compositor_remote_message_server_base::message::Message,
    ) {
        let Some(iced) = self.iced.as_mut() else {
            return;
        };

        match message.Value.into_request().into_inner() {
            ServerEvent::Canvas(CanvasMessage::Notify(notify)) => {
                iced.queue_message(Message::SelectNotify(notify.size))
            }
        }
    }
}

delegate_compositor!(OverlayClient);
delegate_output!(OverlayClient);
delegate_seat!(OverlayClient);
delegate_pointer!(OverlayClient);
delegate_keyboard!(OverlayClient);
delegate_layer!(OverlayClient);
delegate_registry!(OverlayClient);
