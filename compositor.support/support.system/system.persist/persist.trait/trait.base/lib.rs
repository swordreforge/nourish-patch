// The `Persist` trait + the `y5_persist!` macro. A storage opts into persistence
// by implementing `Persist` beside its live type and invoking `y5_persist!` once;
// the macro builds the type-erased `PersistEntry` the system returns from
// `persist()`. Re-export serde_json + the entry types so an owning crate needs
// only THIS crate as a dependency (the macro expands to `$crate::…` paths).
pub use compositor_support_system_persist_entry_base::base::{
    PersistEntry, PersistError, SnapshotOutcome,
};
pub use serde_json;

pub mod base;
