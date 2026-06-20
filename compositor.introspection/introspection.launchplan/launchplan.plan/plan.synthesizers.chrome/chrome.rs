//! Chrome / Chromium family synthesizer.

use std::process::Command;

use compositor_introspection_extraction_window_base::attributes::{ExecArgs, ExecProgram};
use compositor_introspection_extraction_window_base::handlers::chrome::{
    attributes::{AppModeUrl, Incognito, NewWindow, ProfileDirectory, Urls, UserDataDir},
    id as chrome_id,
};
use compositor_introspection_extraction_window_base::HandlerId;

use compositor_introspection_launchplan_plan_core::plan::LaunchPlan;
use compositor_introspection_launchplan_plan_core::synthesizer::LaunchSynthesizer;

pub struct ChromeSynthesizer;

impl LaunchSynthesizer for ChromeSynthesizer {
    fn handler_id(&self) -> HandlerId {
        chrome_id()
    }

    fn synthesize(&self, plan: &LaunchPlan) -> Option<Command> {
        let program = plan.current::<ExecProgram>()?;
        let mut cmd = Command::new(program);

        // Start from the live ExecArgs (user-editable). The user can add
        // arbitrary flags and they flow through here.
        let existing_args: Vec<String> = plan.current::<ExecArgs>().unwrap_or_default();
        for a in &existing_args {
            cmd.arg(a);
        }

        // Append structured Chrome attributes that aren't already present
        // in the live args. We don't replace; we augment.
        if plan.current::<NewWindow>().unwrap_or(true) && !existing_args.iter().any(|a| a == "--new-window") {
            cmd.arg("--new-window");
        }
        if plan.current::<Incognito>().unwrap_or(false) && !existing_args.iter().any(|a| a == "--incognito") {
            cmd.arg("--incognito");
        }
        if let Some(profile) = plan.current::<ProfileDirectory>() {
            if !profile.is_empty() && !existing_args.iter().any(|a| a.starts_with("--profile-directory=")) {
                cmd.arg(format!("--profile-directory={profile}"));
            }
        }
        if let Some(udd) = plan.current::<UserDataDir>() {
            if !existing_args.iter().any(|a| a.starts_with("--user-data-dir=")) {
                cmd.arg(format!("--user-data-dir={}", udd.to_string_lossy()));
            }
        }
        if let Some(app_url) = plan.current::<AppModeUrl>() {
            if !app_url.is_empty() && !existing_args.iter().any(|a| a.starts_with("--app=")) {
                cmd.arg(format!("--app={app_url}"));
            }
        }
        if let Some(urls) = plan.current::<Urls>() {
            // URLs append cleanly; check for any URL-shaped existing arg to dedupe.
            for u in urls {
                if !existing_args.contains(&u) {
                    cmd.arg(u);
                }
            }
        }

        Some(cmd)
    }
}
