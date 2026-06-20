//! JetBrains family synthesizer.

use std::process::Command;

use compositor_introspection_extraction_window_base::attributes::{ExecArgs, ExecProgram};
use compositor_introspection_extraction_window_base::handlers::jetbrains::{
    attributes::ProjectPath, id as jetbrains_id,
};
use compositor_introspection_extraction_window_base::HandlerId;

use compositor_introspection_launchplan_plan_core::plan::LaunchPlan;
use compositor_introspection_launchplan_plan_core::synthesizer::LaunchSynthesizer;

pub struct JetBrainsSynthesizer;

impl LaunchSynthesizer for JetBrainsSynthesizer {
    fn handler_id(&self) -> HandlerId {
        jetbrains_id()
    }

    fn synthesize(&self, plan: &LaunchPlan) -> Option<Command> {
        let program = plan.current::<ExecProgram>()?;
        let mut cmd = Command::new(program);

        let existing_args: Vec<String> = plan.current::<ExecArgs>().unwrap_or_default();
        for a in &existing_args {
            cmd.arg(a);
        }

        if let Some(project_path) = plan.current::<ProjectPath>() {
            let pp = project_path.to_string_lossy().to_string();
            if !existing_args.iter().any(|a| a == &pp) {
                cmd.arg(project_path);
            }
        }

        Some(cmd)
    }
}
