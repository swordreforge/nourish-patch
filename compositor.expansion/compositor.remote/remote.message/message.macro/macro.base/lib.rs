//! Facade: the macros moved to flat sibling crates; every public path
//! (`compositor_remote_message_macro_base::define!` / `define_broadcasts!`)
//! keeps resolving through these re-exports.

pub use compositor_remote_message_macro_broadcast::define_broadcasts;
pub use compositor_remote_message_macro_define::define;
