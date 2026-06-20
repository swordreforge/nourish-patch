use compositor_introspection_extraction_window_handlers_terminal_attributes::attributes::LaunchCwd;
use compositor_introspection_extraction_window_hints_inferred::inferred::InferredHints;
use compositor_introspection_extraction_window_hints_source::source::{Confidence, SourceMethod};
use compositor_introspection_extraction_window_meta_types::types::Meta;
use std::path::PathBuf;

/// Working-directory flags from the terminal's cmdline.
pub fn push_cwd_flags(meta: &Meta, hints: &mut InferredHints) {
    let Some(cmdline) = &meta.cmdline else { return };
    let mut iter = cmdline.iter().skip(1).peekable();
    while let Some(arg) = iter.next() {
        if let Some(v) = arg.strip_prefix("--working-directory=") {
            hints.push::<LaunchCwd>(
                PathBuf::from(v),
                SourceMethod::ProcCmdline,
                "--working-directory flag",
                Confidence::High,
            );
            continue;
        }
        if arg == "--working-directory" {
            if let Some(v) = iter.next() {
                hints.push::<LaunchCwd>(
                    PathBuf::from(v),
                    SourceMethod::ProcCmdline,
                    "--working-directory flag (split)",
                    Confidence::High,
                );
            }
            continue;
        }
        if arg == "--directory" || arg == "-d" {
            if let Some(v) = iter.next() {
                hints.push::<LaunchCwd>(
                    PathBuf::from(v),
                    SourceMethod::ProcCmdline,
                    "kitty --directory flag",
                    Confidence::High,
                );
            }
            continue;
        }
        if arg == "--workdir" {
            if let Some(v) = iter.next() {
                hints.push::<LaunchCwd>(
                    PathBuf::from(v),
                    SourceMethod::ProcCmdline,
                    "konsole --workdir flag",
                    Confidence::High,
                );
            }
            continue;
        }
    }
}
