use compositor_support_system_persist_document_trait::base::Document;
use compositor_support_system_persist_document_trait::y5_document;
use compositor_y5_picker_state_base::base::{PickerState, PICKER, PICKER_MUT};

/// One scene world's persisted identity: its picker display name + grid cell.
/// The record id is the world UUID (so its per-world state reloads under the same
/// id). Kind is implied SPATIAL (the picker only tracks scene worlds).
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WorldRecord {
    pub display_name: String,
    pub cell: usize,
}

/// Projects the picker's world registry into the global `world` table.
pub struct WorldsDoc;

impl Document for WorldsDoc {
    type Slot = PickerState;
    type Record = WorldRecord;
    const TABLE: &'static str = "world";
    const VERSION: u32 = 1;
    const WORLD_PARTITIONED: bool = false; // global registry, not per-world

    fn rows(s: &PickerState) -> Vec<(String, Vec<(&'static str, String)>, WorldRecord)> {
        s.cell_worlds
            .iter()
            .enumerate()
            .filter_map(|(cell, w)| {
                w.map(|id| {
                    let display_name = s.world_names.get(&id).cloned().unwrap_or_default();
                    (id.to_string(), Vec::new(), WorldRecord { display_name, cell })
                })
            })
            .collect()
    }

    fn apply(s: &mut PickerState, id: &str, rec: WorldRecord) {
        if let Ok(uuid) = uuid::Uuid::parse_str(id) {
            if rec.cell < s.cell_worlds.len() {
                s.cell_worlds[rec.cell] = Some(uuid);
            }
            s.world_names.insert(uuid, rec.display_name);
        }
    }
}

y5_document!(WORLDS_DOC, WorldsDoc, PICKER, PICKER_MUT);
