use compositor_support_system_buffer_token_base::y5_buffer;
use compositor_support_system_channel_token_base::y5_channel;
use compositor_support_system_storage_token_base::base::{Token, TokenMut};
use compositor_support_system_storage_slot_base::base::Storage;
use compositor_support_system_trait_system_base::base::{BufferCx, System, SystemCx, WorldBuilder};
use compositor_support_iced_core_engine_base::{EngineSettings, SharedEngine};
use compositor_y5_surface_state_base::state::SurfaceState;
use compositor_monitor_runtime_surface_base::WgpuVulkanContext;
use compositor_monitor_compositor_iced_base::{HandleId, IcedRegistry};
use smithay::utils::{Physical, Point, Size};
use std::any::Any;
use std::sync::Arc;

/// An iced pointer-button event routed to the surface registry. Input systems
/// can't touch the registry (it's this system's slot), so they announce this and
/// the surface system applies it on its own slot. `target: None` = current
/// grab/focus (matches the legacy `dispatch_button(None, …)`).
#[derive(Clone, Copy)]
pub struct IcedButton {
    pub button: u32,
    pub pressed: bool,
}
y5_channel!(pub ICED_BUTTON, ICED_BUTTON_TX: IcedButton);

/// Announce an iced pointer button to the surface system. Cross-crate senders
/// can't reach the channel TX directly (it stays crate-internal, like the
/// navigator's `request()`), so they call this on the world's router.
pub fn announce_iced_button(
    channels: &mut compositor_support_system_channel_router_base::base::ChannelRouter,
    button: u32,
    pressed: bool,
) {
    channels.send(&ICED_BUTTON_TX, IcedButton { button, pressed });
}

/// An iced keyboard-focus change routed to the surface registry. Like
/// `IcedButton`, input systems can't touch the registry, so they announce this
/// and the surface system applies it on its own slot. `target: None` clears
/// keyboard focus (matches the legacy `set_keyboard_focus(None)`).
#[derive(Clone, Copy)]
pub struct IcedFocus {
    pub target: Option<HandleId>,
}
y5_channel!(pub ICED_FOCUS, ICED_FOCUS_TX: IcedFocus);

/// Announce an iced keyboard-focus change to the surface system. Mirrors
/// `announce_iced_button`; cross-crate senders can't reach the channel TX
/// directly, so they call this on the world's router.
pub fn announce_iced_focus(
    channels: &mut compositor_support_system_channel_router_base::base::ChannelRouter,
    target: Option<HandleId>,
) {
    channels.send(&ICED_FOCUS_TX, IcedFocus { target });
}

/// A placeholder iced surface's geometry, routed to the registry. The
/// placeholder slot lives on `PlaceholderSystem`, but the resize/relocate also
/// has to hit the iced REGISTRY (this system's slot) — the second half of the
/// legacy `placeholder.interface::set_visible_geometry`. The placeholder system
/// applies the slot half and announces this for the registry half.
#[derive(Clone, Copy)]
pub struct PlaceholderGeometry {
    pub handle: HandleId,
    pub position: Option<Point<i32, Physical>>,
    pub size: Option<Size<i32, Physical>>,
}
y5_channel!(pub PLACEHOLDER_GEOMETRY, PLACEHOLDER_GEOMETRY_TX: PlaceholderGeometry);

/// Announce a placeholder iced-surface geometry change to the surface system.
/// Mirrors `announce_iced_button`.
pub fn announce_placeholder_geometry(
    channels: &mut compositor_support_system_channel_router_base::base::ChannelRouter,
    handle: HandleId,
    position: Option<Point<i32, Physical>>,
    size: Option<Size<i32, Physical>>,
) {
    channels.send(&PLACEHOLDER_GEOMETRY_TX, PlaceholderGeometry { handle, position, size });
}

enum SurfaceCmd {
    Button(u32, bool),
    Focus(Option<HandleId>),
    PlaceholderGeometry(HandleId, Option<Point<i32, Physical>>, Option<Size<i32, Physical>>),
}
y5_buffer!(SURF_BUF: SurfaceCmd);

pub static SURFACE: Token<SurfaceState> = Token::new();
/// TRANSITIONAL pub: legacy call sites still write this slot directly until
/// their logic moves into systems/events (pass 2 of phase 4).
pub static SURFACE_MUT: TokenMut<SurfaceState> = TokenMut::new(&SURFACE);

/// Shared iced GPU context (KERNEL driver data, not per-world). The wgpu
/// device/queue/adapter is a single shared resource: the loader blocks on the
/// async wgpu init at startup and stores it here ONCE, then every world's
/// `SurfaceState` builds its OWN `IcedRegistry` from it via `ensure_registry`
/// (per-world surfaces, shared GPU) at world-build time — never during render.
/// This is what lets a non-main world have a registry at all.
pub static ICED_CONTEXT: Token<Option<Arc<WgpuVulkanContext>>> = Token::new();
pub static ICED_CONTEXT_MUT: TokenMut<Option<Arc<WgpuVulkanContext>>> = TokenMut::new(&ICED_CONTEXT);

/// Build THIS world's iced registry from the shared kernel context, off the
/// render path. No-op if the world doesn't run `SurfaceSystem` (no `SURFACE`
/// slot — e.g. an overlay world), already has a registry, or the context hasn't
/// landed. Called at world build / startup prewarm so the registry is asserted
/// present by the time a frame draws, rather than lazily constructed mid-render.
pub fn ensure_registry(storage: &mut Storage, kernel: &Storage) {
    let Some(surface) = storage.try_get_mut(&SURFACE_MUT) else {
        return;
    };
    if surface.registry.is_some() {
        return;
    }
    let Some(wgpu) = kernel.try_get(&ICED_CONTEXT).and_then(|c| c.clone()) else {
        return;
    };
    info!("prewarm: build per-world iced registry");
    let shared_engine = SharedEngine::new(
        &wgpu.adapter,
        Arc::new(wgpu.device.clone()),
        Arc::new(wgpu.queue.clone()),
        compositor_monitor_runtime_surface_base::TEXTURE_FORMAT,
        EngineSettings::default(),
    );
    surface.registry = Some(IcedRegistry::new(shared_engine, wgpu));
}

/// Owns the surface slot.
#[derive(Default)]
pub struct SurfaceSystem;

impl System for SurfaceSystem {
    fn name(&self) -> &'static str {
        "surface"
    }

    fn register(&mut self, builder: &mut WorldBuilder) {
        builder.storage.insert(&SURFACE, SurfaceState::new());
        builder.receive(&ICED_BUTTON, Self::on_iced_button);
        builder.receive(&ICED_FOCUS, Self::on_iced_focus);
        builder.receive(&PLACEHOLDER_GEOMETRY, Self::on_placeholder_geometry);
    }

    fn buffer(&mut self, cx: &mut BufferCx, message: Box<dyn Any>) {
        match *message.downcast::<SurfaceCmd>().expect("surface buffer type") {
            SurfaceCmd::Button(button, pressed) => {
                if let Some(reg) = cx.storage.get_mut(&SURFACE_MUT).registry.as_mut() {
                    reg.dispatch_button(None, button, pressed);
                }
            }
            SurfaceCmd::Focus(target) => {
                if let Some(reg) = cx.storage.get_mut(&SURFACE_MUT).registry.as_mut() {
                    reg.set_keyboard_focus(target);
                }
            }
            SurfaceCmd::PlaceholderGeometry(handle, position, size) => {
                if let Some(reg) = cx.storage.get_mut(&SURFACE_MUT).registry.as_mut() {
                    // Mirrors `placeholder.interface::set_visible_geometry`'s
                    // registry half (size first, then location).
                    if let Some(size) = size {
                        reg.request_resize_by_id(handle, size);
                    }
                    if let Some(position) = position {
                        reg.set_location_by_id(handle, position);
                    }
                }
            }
        }
    }
}

impl SurfaceSystem {
    /// Announced by an input system on release; routed to the registry via the
    /// buffer (the only mutation path) on this system's own slot.
    fn on_iced_button(&mut self, cx: &mut SystemCx, ev: &IcedButton) {
        cx.write(&SURF_BUF, SurfaceCmd::Button(ev.button, ev.pressed));
    }

    /// Announced by an input system on press (to clear keyboard focus on a
    /// canvas click); routed to the registry via the buffer.
    fn on_iced_focus(&mut self, cx: &mut SystemCx, ev: &IcedFocus) {
        cx.write(&SURF_BUF, SurfaceCmd::Focus(ev.target));
    }

    /// Announced by the placeholder system during an interactive placeholder
    /// move/scale; routed to the iced registry via the buffer.
    fn on_placeholder_geometry(&mut self, cx: &mut SystemCx, ev: &PlaceholderGeometry) {
        cx.write(&SURF_BUF, SurfaceCmd::PlaceholderGeometry(ev.handle, ev.position, ev.size));
    }
}
