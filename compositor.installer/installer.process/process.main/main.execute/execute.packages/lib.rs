//! Installer step 1: detect the GPU, prompt the dnf package/driver groups, and
//! install the selection.

use compositor_installer_process_config_parse_base::prompt;
use compositor_installer_process_packages_enumerate_base as pkg;

/// Detect the GPU vendor, prompt every package group (Enter keeps the default),
/// and run dnf over the selection. A failed install is a warning, not fatal.
pub fn select_and_install(dry_run: bool) {
    let gpu = pkg::detect_gpu();
    println!("\nDetected GPU vendor: {gpu:?}");
    let mut selected_packages: Vec<String> = Vec::new();
    println!("\n-- Package groups (Enter keeps the [default]) --");
    for group in pkg::groups(gpu) {
        let want = prompt::yes_no(
            group.key,
            &format!("{} — {}", group.title, group.description),
            group.default_on,
        );
        if want {
            selected_packages.extend(group.packages.iter().map(|s| s.to_string()));
        }
    }
    if selected_packages.is_empty() {
        println!("No package groups selected — skipping dnf.");
    } else {
        println!("\nInstalling {} packages via dnf...", selected_packages.len());
        if let Err(e) = pkg::dnf_install(&selected_packages, dry_run) {
            eprintln!("warning: package install failed: {e}");
        }
    }
}
