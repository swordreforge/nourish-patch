// The `Document` trait + the `y5_document!` macro. A collection slot opts into the
// filesystem table store by implementing `Document` (projecting its records) and
// invoking `y5_document!`. Re-exports the entry types + serde_json so an owning
// crate needs only THIS crate as a dependency.
pub use compositor_support_system_persist_document_entry::base::{DocRow, DocumentEntry};
pub use compositor_support_system_persist_entry_base::base::PersistError;
pub use serde_json;

pub mod base;
