//! Facade: gRPC service handler bodies live in flat sibling crates
//! (`handle.navigator`, `handle.selection`, `handle.aspect`, `handle.debug`,
//! `handle.stack`); this crate keeps the `Handle` type and the public
//! `handle::execute` entry point.

pub mod handle;
