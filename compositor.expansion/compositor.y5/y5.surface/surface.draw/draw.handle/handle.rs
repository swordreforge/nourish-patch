use smithay::backend::renderer::gles::GlesRenderer;
use smithay::utils::{Logical, Physical, Point, Rectangle, Size};
use std::sync::{Arc, mpsc};
use compositor_orchestration_core_state_base::Loop;
use compositor_monitor_compositor_iced_base::{IcedHandle, IcedRegistry};

use compositor_support_iced_core_engine_base::{EngineSettings, IcedUi, SharedEngine};

pub use compositor_monitor_compositor_iced_base::IcedSpace;

pub fn load<T: IcedUi>(
    state: &mut Loop,
    gles: &mut GlesRenderer,
    t: T,
    rect: Rectangle<i32, Physical>,
    space: IcedSpace,
    layer: u64,
) -> IcedHandle<T> {
    // Hoist GPU before the surface-registry borrow (surface_mut borrows all of inner).
    let gpu = state.inner.environment.GPU.clone();
    let registry = state.inner.surface_mut()
        .registry
        .as_mut()
        .unwrap_or_else(|| abort!("registry to be created"));

    let handle = match space {
        IcedSpace::World => registry
            .create(
                &gpu.as_str(),
                t,
                gles,
                rect.loc,
                rect.size,
                layer,
            )
            .unwrap(),
        IcedSpace::Screen => registry
            .create_screen(
                &gpu.as_str(),
                t,
                gles,
                rect.loc,
                rect.size,
                layer,
            )
            .unwrap(),
    };

    // Register WORLD-space iced surfaces in the renderer-agnostic draw order so
    // they interleave with windows by DrawOrder ("everything" interleaves;
    // screen-space iced is a screen-locked overlay kept on its own band). The id
    // is derived from the iced HandleId (reversible: HandleId(uuid.as_u128() as u64)).
    if let IcedSpace::World = space {
        // Map the surface's layer mask to a draw-order tier: group frames sit
        // beneath the windows they contain; everything else is CONTENT.
        let tier = if layer & compositor_orchestration_draw_layer_base::base::Layer::SCENE_SURFACE_GROUP.bits() != 0 {
            compositor_support_world_order_track_base::base::DrawLayer::GROUP
        } else {
            compositor_support_world_order_track_base::base::DrawLayer::CONTENT
        };
        state.inner.register_drawable(uuid::Uuid::from_u128(handle.id.0 as u128), tier);
    }

    // Install the message observer.
    // CHECK: This is per-manager callback.
    // CHECK: Rendering is already auto-managed.
    // messages should already be populated inside the calloop and not on a different thread.
    // however, messages should be buffered and taken out, or must be explicitly handled for the surface to re-render.
    // this needs to be a toggle.
    // eg. it wont re-render until the owner of IcedHandle<T> will handle the incoming buffer and remain stale.

    handle
}
