//! Interactive y5 installer.
//!
//! Run from the unzipped artifact (the prebuilt binaries + templates sit next to
//! this executable). It:
//!   1. detects the GPU and installs the chosen dnf package/driver groups,
//!   2. prompts the default Y5 Desktop configuration (values propagate to every preset),
//!   3. lays down all session presets (renderer × experimental × sync, + Custom),
//!   4. optionally installs the dev tool window, MX daemon, polkit agent and PAM lock.
//!
//! Re-runnable: every file is overwritten. `--dry-run` prints the plan without
//! touching the system. The steps live in the sibling `execute.*` crates.

use compositor_installer_process_layout_compute_base as layout;
use compositor_installer_process_main_execute_configure as configure;
use compositor_installer_process_main_execute_info as info;
use compositor_installer_process_main_execute_packages as packages;
use compositor_installer_process_main_execute_plan as plan;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dry_run = args.iter().any(|a| a == "--dry-run" || a == "-n");
    if args.iter().any(|a| a == "-h" || a == "--help") {
        info::print_help();
        return;
    }
    if args.iter().any(|a| a == "--emit-presets") {
        info::emit_presets();
        return;
    }

    println!("=== y5 compositor installer ===");
    if dry_run {
        println!("(dry-run: no changes will be made)\n");
    }

    let stage = layout::Stage::resolve();
    println!("Artifact staging dir: {}", stage.root.display());

    // 1) Packages / drivers.
    packages::select_and_install(dry_run);

    // 2-3) Default Y5 Desktop configuration + presets (incl. optional Custom).
    let presets = configure::gather();

    // 4-5) Optional components, plan assembly, apply.
    plan::build_and_apply(&stage, &presets, dry_run);
}
