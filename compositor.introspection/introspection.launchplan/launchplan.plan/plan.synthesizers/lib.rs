//! Built-in launch synthesizers, one per built-in handler.

pub mod synthesizers;

pub use synthesizers::{default_synthesizers, register_builtin_synthesizers};
