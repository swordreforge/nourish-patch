//! Prewarm a world's OWN bevy registry from the shared kernel context, for rim
//! callers holding `&mut Storage` directly (the picker/lock scenes own their
//! registry rather than borrowing main's). Mirrors `ThreeSystem::update`'s build
//! but writes the slot directly. Requires the world to have `ThreeSystem`
//! registered (so `BG_THREE` exists).

use compositor_background_three_system_base::base::{BG_THREE, BG_THREE_MUT, BEVY_CONTEXT};
use compositor_support_bevy_core_compositor_base::{BevyRegistry, SharedContext};
use compositor_support_system_storage_slot_base::base::Storage;

pub fn ensure_registry(storage: &mut Storage, kernel: &Storage) {
    // No-op if this world doesn't run `ThreeSystem` (no `BG_THREE` slot) or it
    // already has a registry — keeps the startup prewarm pass uniform across
    // worlds that do and don't host 3D.
    let Some(state) = storage.try_get_mut(&BG_THREE_MUT) else {
        return;
    };
    if state.registry.is_some() {
        return;
    }
    let Some(ctx) = kernel.try_get(&BEVY_CONTEXT).and_then(|c| c.clone()) else {
        return;
    };
    info!("Prewarm: build per-world bevy registry");
    let shared = SharedContext::new(
        ctx.instance.clone(),
        ctx.adapter.clone(),
        ctx.device.clone(),
        ctx.queue.clone(),
    );
    state.shared = Some(shared.clone());
    state.registry = Some(BevyRegistry::new(shared, ctx));
}
