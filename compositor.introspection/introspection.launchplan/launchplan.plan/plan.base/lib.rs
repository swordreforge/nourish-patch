//! # compositor_introspection_launchplan_plan_base
//!
//! Façade: persistent launch plan — application data + per-handler
//! preferences + executable synthesis. Implementation lives in the
//! sibling `plan.*` crates; this crate re-exports the full public tree.

pub mod plan {
    pub use compositor_introspection_launchplan_plan_core::plan::*;
}
pub mod preferences {
    pub use compositor_introspection_launchplan_plan_preferences::preferences::*;
}
pub mod synthesizer {
    pub use compositor_introspection_launchplan_plan_core::synthesizer::*;
}
pub mod synthesizers {
    pub use compositor_introspection_launchplan_plan_synthesizers::synthesizers::*;
}
pub mod exec {
    pub use compositor_introspection_launchplan_plan_exec_opts::exec::*;
    pub use compositor_introspection_launchplan_plan_exec_run::exec::*;
    pub use compositor_introspection_launchplan_plan_exec_spawn::exec::*;
    pub use compositor_introspection_launchplan_plan_exec_unit::exec::*;
}

pub use plan::LaunchPlan;
pub use preferences::{PreferenceField, Preferences};
pub use synthesizer::{LaunchSynthesizer, SynthesizerRegistry};
pub use synthesizers::default_synthesizers;

pub use compositor_introspection_extraction_window_base::HandlerId;
pub use compositor_introspection_inference_hint_base::ApplicationData;
