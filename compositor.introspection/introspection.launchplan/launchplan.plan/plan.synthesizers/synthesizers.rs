//! Built-in launch synthesizers, one per built-in handler.

pub use compositor_introspection_launchplan_plan_synthesizers_chrome::chrome;
pub use compositor_introspection_launchplan_plan_synthesizers_jetbrains::jetbrains;
pub use compositor_introspection_launchplan_plan_synthesizers_nautilus::nautilus;
pub use compositor_introspection_launchplan_plan_synthesizers_terminal::terminal;

use compositor_introspection_launchplan_plan_core::synthesizer::SynthesizerRegistry;

/// Register all built-in synthesizers.
pub fn register_builtin_synthesizers(registry: &mut SynthesizerRegistry) {
    registry.register(chrome::ChromeSynthesizer);
    registry.register(jetbrains::JetBrainsSynthesizer);
    registry.register(nautilus::NautilusSynthesizer);
    registry.register(terminal::TerminalSynthesizer);
    // No GenericSynthesizer: when no synthesizer matches the active
    // handler (Generic or otherwise), LaunchPlan::execute falls back
    // to generic exec automatically.
}

/// Convenience: a fresh registry with all builtins registered.
pub fn default_synthesizers() -> SynthesizerRegistry {
    let mut r = SynthesizerRegistry::new();
    register_builtin_synthesizers(&mut r);
    r
}
