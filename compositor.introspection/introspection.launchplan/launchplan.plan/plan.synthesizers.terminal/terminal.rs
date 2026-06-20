//! Terminal emulator synthesizer. Reads the terminal kind (preferring
//! `ForegroundCwd`, falling back to `LaunchCwd`) and emits the right
//! per-flavor working-directory flag.

use std::process::Command;

use compositor_introspection_extraction_window_base::attributes::{ExecArgs, ExecProgram};
use compositor_introspection_extraction_window_base::handlers::terminal::{
    attributes::{ForegroundCwd, LaunchCwd, TerminalKind, TerminalKindAttr},
    id as terminal_id,
};
use compositor_introspection_extraction_window_base::HandlerId;

use compositor_introspection_launchplan_plan_core::plan::LaunchPlan;
use compositor_introspection_launchplan_plan_core::synthesizer::LaunchSynthesizer;

pub struct TerminalSynthesizer;

impl LaunchSynthesizer for TerminalSynthesizer {
    fn handler_id(&self) -> HandlerId {
        terminal_id()
    }

    fn synthesize(&self, plan: &LaunchPlan) -> Option<Command> {
        let program = plan.current::<ExecProgram>()?;
        let mut cmd = Command::new(program);

        let existing_args: Vec<String> = plan.current::<ExecArgs>().unwrap_or_default();
        for a in &existing_args {
            cmd.arg(a);
        }

        let cwd = plan
            .current::<ForegroundCwd>()
            .or_else(|| plan.current::<LaunchCwd>());

        let kind = plan
            .current::<TerminalKindAttr>()
            .unwrap_or(TerminalKind::Unknown(String::new()));

        // Only emit per-terminal-flavor working-directory flag if none of
        // the well-known variants is already in the args.
        let has_wd_flag = existing_args.iter().any(|a| {
            a.starts_with("--working-directory")
                || a.starts_with("--directory")
                || a == "-d"
                || a.starts_with("--workdir")
                || a == "--cwd"
        });

        if let Some(cwd) = cwd {
            if !has_wd_flag {
                let cwd_str = cwd.to_string_lossy().to_string();
                match kind {
                    TerminalKind::Alacritty => {
                        cmd.arg("--working-directory").arg(&cwd_str);
                    }
                    TerminalKind::Foot => {
                        cmd.arg(format!("--working-directory={cwd_str}"));
                    }
                    TerminalKind::GnomeTerminal => {
                        cmd.arg(format!("--working-directory={cwd_str}"));
                    }
                    TerminalKind::GnomeConsole | TerminalKind::Ptyxis => {
                        cmd.arg("--working-directory").arg(&cwd_str);
                    }
                    TerminalKind::Kitty => {
                        cmd.arg("--directory").arg(&cwd_str);
                    }
                    TerminalKind::WezTerm => {
                        cmd.arg("start").arg("--cwd").arg(&cwd_str);
                    }
                    TerminalKind::Konsole => {
                        cmd.arg("--workdir").arg(&cwd_str);
                    }
                    TerminalKind::Xterm => {
                        cmd.arg("-e").arg("sh").arg("-c").arg(format!(
                            "cd {} && exec ${{SHELL:-/bin/sh}}",
                            shell_escape(&cwd_str)
                        ));
                    }
                    TerminalKind::Unknown(_) => {}
                }
            }
        }

        Some(cmd)
    }
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
