//! Each handler submodule defines a marker type, an `attributes` submodule,
//! the `AppHandler` impl and a `register(&mut HandlerRegistry)` function.
//! External crates follow the same pattern; call their `register` alongside
//! these to plug them in.

pub use compositor_introspection_extraction_window_handlers_chrome::chrome;
pub use compositor_introspection_extraction_window_handlers_generic::generic;
pub use compositor_introspection_extraction_window_handlers_jetbrains::jetbrains;
pub use compositor_introspection_extraction_window_handlers_nautilus::nautilus;
pub use compositor_introspection_extraction_window_handlers_terminal::terminal;

use compositor_introspection_extraction_window_handler_registry::registry::HandlerRegistry;

/// Register all built-in handlers (Generic + the four target apps) and
/// mark Generic as the fallback.
pub fn register_builtin_handlers(registry: &mut HandlerRegistry) {
    generic::register(registry); // also sets the fallback
    nautilus::register(registry);
    jetbrains::register(registry);
    chrome::register(registry);
    terminal::register(registry);
}

/// Convenience: a fresh registry with all builtins registered.
pub fn default_registry() -> HandlerRegistry {
    let mut registry = HandlerRegistry::new();
    register_builtin_handlers(&mut registry);
    registry
}
