use compositor_orchestration_core_state_base::Loop;
use compositor_y5_picker_system_base::base::{PICKER, PICKER_WORLD};

/// Recreate the scene worlds restored into the picker grid from the `world` table
/// (whose ids the picker world's build loaded into `cell_worlds`). Each missing
/// world is rebuilt under its saved id so its state + placeholders reload. Call
/// once at startup after the `WorldManager` is assembled.
pub fn restore_worlds(state: &mut Loop) {
    let saved: Vec<uuid::Uuid> = state
        .inner
        .worlds
        .get(PICKER_WORLD)
        .storage()
        .get(&PICKER)
        .cell_worlds
        .iter()
        .flatten()
        .copied()
        .collect();
    let count = saved.len();
    for id in saved {
        if !state.inner.worlds.contains(id) {
            compositor_y5_picker_world_base::base::create_world_with_id(state, id);
        }
    }
    if count > 0 {
        info!("picker: restored {count} world(s) from disk");
    }
}
