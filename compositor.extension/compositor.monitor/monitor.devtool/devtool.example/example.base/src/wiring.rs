//! Example compositor-side wiring for `CounterUi`.
//!
//! This file is *commented for reading*, not for running — it references
//! `gles_renderer`, `state`, and other things your compositor binary owns
//! and that aren't constructable from this example crate.
//!
//! Copy the relevant snippets into your compositor binary.

#![allow(dead_code, unused_variables)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use compositor_support_iced_core_engine_base::{
    EngineSettings, IcedRuntime, MessageHandler, SharedEngine,
};
// In your real compositor binary, also import:
//   use compositor_monitor_compositor_iced_base::{IcedRegistry, IcedHandle};
//   use compositor_monitor_runtime_surface_base::{create_wgpu_vulkan_context, TEXTURE_FORMAT};

use crate::counter_ui::{CounterUi, OutgoingMessage};

// ─────────────────────────────────────────────────────────────────────
// Step 1: One-time initialization in your compositor's startup
// ─────────────────────────────────────────────────────────────────────
//
// In your real compositor, this goes alongside the existing Bevy WGPU
// context init in `integration.md`. Use the same pattern: dispatch to a
// background thread, receive via mpsc.
//
// pub fn setup_iced_subsystem(state: &mut State, gles: &mut GlesRenderer) {
//     // Already arrived from a background thread, per the Bevy pattern:
//     let wgpu_ctx = state.iced_wgpu_context
//         .take()
//         .expect("Iced wgpu context must be ready");
//
//     let shared_engine = SharedEngine::new(
//         &wgpu_ctx.adapter,
//         Arc::new(wgpu_ctx.device.clone()),
//         Arc::new(wgpu_ctx.queue.clone()),
//         TEXTURE_FORMAT,
//         EngineSettings::default(),
//     );
//
//     state.iced_registry = Some(IcedRegistry::new(shared_engine, Arc::new(wgpu_ctx)));
// }

// ─────────────────────────────────────────────────────────────────────
// Step 2: Create the CounterUi instance and install a message handler
// ─────────────────────────────────────────────────────────────────────
//
// pub fn spawn_counter_panel(state: &mut State, gles: &mut GlesRenderer)
//     -> IcedHandle<CounterUi>
// {
//     let registry = state.iced_registry.as_mut().unwrap();
//
//     // Place a 360×220 panel near top-left of the screen.
//     let handle = registry.create(
//         CounterUi::default(),
//         gles,
//         smithay::utils::Point::from((40, 40)),
//         smithay::utils::Size::from((360, 220)),
//     ).expect("create counter UI");
//
//     // Install the message handler that observes every message the UI
//     // emits (Iced → Smithay direction).
//     let inst = registry.instance_mut(handle).unwrap();
//     inst.runtime_mut().set_message_handler(observe_outgoing);
//
//     handle
// }

/// Stand-alone message observer. The handler runs synchronously when each
/// message is drained out of the runtime queue, before `update` applies it.
///
/// This is the place where you'd dispatch to other subsystems: shell out a
/// command, fire a custom Wayland protocol message, mutate compositor state,
/// etc. Whatever you'd otherwise do in response to user interaction.
fn observe_outgoing(message: &OutgoingMessage) {
    match message {
        OutgoingMessage::IncrementClicked => {
            info!("compositor saw IncrementClicked");
            // e.g., state.bump_some_counter();
        }
        OutgoingMessage::ResetClicked => {
            info!("compositor saw ResetClicked");
            // e.g., state.broadcast_reset();
        }
        // Other variants are inbound (compositor → UI). The UI's queue still
        // receives them, and the handler still sees them — useful for
        // logging round-trip latency or similar. You don't *have* to act on
        // them; the UI itself will do that in `update`.
        OutgoingMessage::SmithayTick(_)
        | OutgoingMessage::SmithayReset
        | OutgoingMessage::SmithayLabel(_) => {}
        OutgoingMessage::ItemClicked(_) => {}
        OutgoingMessage::TextInputChanged(_) => {}
        OutgoingMessage::TextInputSubmitted => {}
    }
}

// ─────────────────────────────────────────────────────────────────────
// Step 3: Dispatch from the compositor INTO the UI
// ─────────────────────────────────────────────────────────────────────
//
// Three example sources:
//   3a. A periodic timer firing SmithayTick.
//   3b. A keybind firing SmithayReset.
//   3c. An ad-hoc status update via SmithayLabel.

// 3a. Periodic tick. Drive this from your event loop / calloop timer.
//
// Pseudocode for inside your frame callback:
//
// pub fn tick_iced(state: &mut State, handle: IcedHandle<CounterUi>) {
//     let now = Instant::now();
//     if now.duration_since(state.last_iced_tick) >= Duration::from_secs(1) {
//         state.last_iced_tick = now;
//         state.iced_tick_counter += 1;
//         let _ = state.iced_registry.as_mut().unwrap()
//             .dispatch_message(handle, OutgoingMessage::SmithayTick(state.iced_tick_counter));
//     }
// }

// 3b. Keybind handler. Plug into your existing keyboard handling.
//
// pub fn on_key_F2(state: &mut State, handle: IcedHandle<CounterUi>) {
//     let _ = state.iced_registry.as_mut().unwrap()
//         .dispatch_message(handle, OutgoingMessage::SmithayReset);
// }

// 3c. Ad-hoc label update.
//
// pub fn report_status(state: &mut State, handle: IcedHandle<CounterUi>, msg: impl Into<String>) {
//     let _ = state.iced_registry.as_mut().unwrap()
//         .dispatch_message(handle, OutgoingMessage::SmithayLabel(msg.into()));
// }

// ─────────────────────────────────────────────────────────────────────
// Step 4: Per-frame processing (in your render callback)
// ─────────────────────────────────────────────────────────────────────
//
// Where you currently produce the BevyBackgroundElement, also do:
//
// pub fn produce_iced_elements(state: &mut State, gles: &mut GlesRenderer)
//     -> Vec<IcedRenderElement>
// {
//     state.iced_registry
//         .as_mut()
//         .unwrap()
//         .render_all(gles)
//         .unwrap_or_else(|e| {
//             tracing::warn!(error = ?e, "iced render_all failed");
//             Vec::new()
//         })
// }
//
// Add the returned elements to the render list alongside windows + Bevy bg.

// ─────────────────────────────────────────────────────────────────────
// Step 5: Pointer routing
// ─────────────────────────────────────────────────────────────────────
//
// In your pointer-motion handler, BEFORE forwarding to wayland clients:
//
// pub fn on_pointer_motion(state: &mut State, point: Point<f64, Physical>) -> bool {
//     if let Some(handle_id) = state.iced_registry.as_mut().unwrap().dispatch_pointer_at(point) {
//         // The pointer is over an iced instance.
//         // Swallow: return without forwarding to wayland clients.
//         return true;
//     }
//     false
// }
//
// For button presses / releases, look up the current pointer target and
// route there:
//
// pub fn on_pointer_button(
//     state: &mut State,
//     linux_code: u32,
//     pressed: bool,
// ) -> bool {
//     let registry = state.iced_registry.as_mut().unwrap();
//     if let Some(target) = registry.pointer_target() {
//         let event = if pressed {
//             compositor_monitor_compositor_iced_base::input::button_pressed(linux_code)
//         } else {
//             compositor_monitor_compositor_iced_base::input::button_released(linux_code)
//         };
//         if let Some(e) = event {
//             let _ = registry.dispatch_event(target, e);
//         }
//         return true;  // swallow
//     }
//     false
// }
//
// For scroll:
//
// pub fn on_scroll(
//     state: &mut State,
//     discrete_x: i32,
//     discrete_y: i32,
//     pixel_x: f64,
//     pixel_y: f64,
// ) -> bool {
//     let registry = state.iced_registry.as_mut().unwrap();
//     if let Some(target) = registry.pointer_target() {
//         if let Some(e) = compositor_monitor_compositor_iced_base::input::wheel_scrolled(
//             discrete_x, discrete_y, pixel_x, pixel_y,
//         ) {
//             let _ = registry.dispatch_event(target, e);
//         }
//         return true;  // swallow
//     }
//     false
// }

// ─────────────────────────────────────────────────────────────────────
// Step 6 (optional): Closure-based observer instead of named function
// ─────────────────────────────────────────────────────────────────────
//
// If you need access to compositor state from inside the message handler,
// use a closure. Note that the closure runs on the same thread as render
// — `state` access must come via shared interior-mutable types or a sender:
//
//     let tx = state.events_tx.clone();
//     inst.runtime_mut().set_message_handler(move |msg: &OutgoingMessage| {
//         let _ = tx.send(SomeAppEvent::FromIced(msg.clone()));
//     });
//
// Then your normal event loop picks up SomeAppEvent::FromIced and mutates
// state in the usual way, without recursive borrows.

/// Smoke-test the message handler API works without a compositor present.
/// Useful for type-checking and a sanity check.
pub fn smoke_test_handler_compiles<H: MessageHandler<OutgoingMessage>>(_h: H) {
    // Just exists to prove the bounds line up.
}

pub fn smoke_test_runtime_construction_signature(engine: SharedEngine) {
    let _rt: IcedRuntime<CounterUi> =
        IcedRuntime::new(CounterUi::default(), engine, (360, 220), 1.0);
    // Don't actually run anything — we'd need a real GPU and texture view.
}
