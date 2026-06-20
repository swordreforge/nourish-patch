//! Nautilus synthesizer.

use std::process::Command;

use compositor_introspection_extraction_window_base::attributes::{ExecArgs, ExecProgram};
use compositor_introspection_extraction_window_base::handlers::nautilus::{
    attributes::LocationUri, id as nautilus_id,
};
use compositor_introspection_extraction_window_base::HandlerId;

use compositor_introspection_launchplan_plan_core::plan::LaunchPlan;
use compositor_introspection_launchplan_plan_core::synthesizer::LaunchSynthesizer;

pub struct NautilusSynthesizer;

impl LaunchSynthesizer for NautilusSynthesizer {
    fn handler_id(&self) -> HandlerId {
        nautilus_id()
    }

    fn synthesize(&self, plan: &LaunchPlan) -> Option<Command> {
        let program = plan.current::<ExecProgram>()?;
        let mut cmd = Command::new(program);

        let existing_args: Vec<String> = plan.current::<ExecArgs>().unwrap_or_default();
        for a in &existing_args {
            cmd.arg(a);
        }

        if let Some(uri) = plan.current::<LocationUri>() {
            if !uri.is_empty() && !existing_args.iter().any(|a| a == &uri) {
                cmd.arg(uri);
            }
        }

        Some(cmd)
    }
}
