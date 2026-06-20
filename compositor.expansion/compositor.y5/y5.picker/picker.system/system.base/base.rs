use compositor_support_system_trait_system_base::base::{System, WorldBuilder};
use compositor_y5_picker_state_base::base::PickerState;

/// The world-selection screen's fixed overlay world id (the single source is
/// `WorldManager`). The picker is an OVERLAY world like lock: no `Space`, no
/// windows. Opening it is a `WorldManager::switch` that fires `on_disable` on the
/// session systems and `on_enable` here.
pub use compositor_orchestration_world_manager_base::manager::PICKER_WORLD;

/// Re-export the state slot tokens (defined cycle-free in `picker.state`) so the
/// legacy interface / scene / seat paths can resolve them through this crate.
pub use compositor_y5_picker_state_base::base::{PICKER, PICKER_MUT};

/// Owns the world-selection screen state slot — registered in the PICKER world,
/// not main. While the picker is on screen the session world is suspended (its
/// systems got `on_disable`) but kept intact; cancelling switches back.
#[derive(Default)]
pub struct PickerSystem;

impl System for PickerSystem {
    fn name(&self) -> &'static str {
        "picker"
    }

    fn register(&mut self, builder: &mut WorldBuilder) {
        builder.storage.insert(&PICKER, PickerState::new());
    }

    /// The global `world` table: the picker owns the scene-world registry, so its
    /// changes (create/rename/delete a world) persist the world records.
    fn documents(
        &self,
    ) -> &'static [&'static compositor_support_system_persist_document_entry::base::DocumentEntry] {
        WORLD_DOCS
    }
}

static WORLD_DOCS: &[&compositor_support_system_persist_document_entry::base::DocumentEntry] =
    &[&compositor_y5_picker_persist_world::base::WORLDS_DOC];
