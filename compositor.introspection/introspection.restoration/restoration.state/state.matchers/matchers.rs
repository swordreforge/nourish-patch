//! Built-in restoration matchers + the convenience constructor.

pub use compositor_introspection_restoration_state_matchers_chrome::chrome;
pub use compositor_introspection_restoration_state_matchers_generic::generic;
pub use compositor_introspection_restoration_state_matchers_jetbrains::jetbrains;
pub use compositor_introspection_restoration_state_matchers_nautilus::nautilus;
pub use compositor_introspection_restoration_state_matchers_terminal::terminal;

use compositor_introspection_restoration_state_registry::registry::MatcherRegistry;

/// Register all built-in matchers and set Generic as the fallback.
pub fn register_builtin_matchers(registry: &mut MatcherRegistry) {
    registry.register(chrome::ChromeMatcher);
    registry.register(jetbrains::JetBrainsMatcher);
    registry.register(nautilus::NautilusMatcher);
    registry.register(terminal::TerminalMatcher);
    registry.set_fallback(generic::GenericMatcher);
}

/// Convenience: a fresh registry with all builtins registered.
pub fn default_matchers() -> MatcherRegistry {
    let mut r = MatcherRegistry::new();
    register_builtin_matchers(&mut r);
    r
}
