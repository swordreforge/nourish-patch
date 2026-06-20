use compositor_support_system_persist_document_entry::base::DocumentEntry;
use compositor_support_system_persist_entry_base::base::PersistEntry;
use compositor_support_system_storage_slot_base::base::Storage;
use uuid::Uuid;

/// Rehydrate a world at build time: its single-value slots, then its document
/// tables (per-record, partition-filtered by world). No-op for empty lists.
pub fn rehydrate_world(
    world: Uuid,
    storage: &mut Storage,
    slots: &[&'static PersistEntry],
    docs: &[&'static DocumentEntry],
) {
    if !slots.is_empty() {
        compositor_support_system_persist_rehydrate_base::base::rehydrate_storage(world, storage, slots);
    }
    if !docs.is_empty() {
        compositor_support_system_persist_document_rehydrate::base::rehydrate_world_documents(docs, storage, world);
    }
}
