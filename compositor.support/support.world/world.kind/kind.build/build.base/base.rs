use compositor_support_smithay_state_space_base::state::SpaceState;
use compositor_support_system_storage_slot_base::base::Storage;
use compositor_support_system_trait_system_base::base::System;
use compositor_support_system_world_host_base::base::World;
use compositor_support_world_host_space_base::base::{SpaceHost, SPACE};

/// Build a SPATIAL world: it hosts a window `Space` (seeded empty; the output is
/// mapped in post-init) and implements `WindowHost` via the SPACE slice. One per
/// monitor. The feature systems are injected by the caller (the loader knows the
/// concrete set); this only stamps the world *kind* (document/ARCHITECTURE.md →
/// "Window tracking").
pub fn spatial(id: uuid::Uuid, name: &'static str, systems: Vec<Box<dyn System>>, kernel: &Storage) -> World {
    let mut world = World::build(id, name, systems, kernel);
    world
        .storage_mut()
        .insert(&SPACE, SpaceHost::new(SpaceState { state: smithay::desktop::Space::default() }));
    // The per-world draw-order authority (window/iced/bevy interleave).
    world.storage_mut().insert(
        &compositor_support_world_order_track_base::base::DRAW_ORDER,
        compositor_support_world_order_track_base::base::DrawOrder::new(),
    );
    world
}

/// Build an OVERLAY world: no `Space`, no `WindowHost` (lock, selection). It does
/// not manage client windows; the spatial world's spawn-target keeps them.
pub fn overlay(id: uuid::Uuid, name: &'static str, systems: Vec<Box<dyn System>>, kernel: &Storage) -> World {
    World::build(id, name, systems, kernel)
}
