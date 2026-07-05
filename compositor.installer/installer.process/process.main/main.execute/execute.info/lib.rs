//! Non-interactive installer outputs: `--emit-presets` and `--help`.

use compositor_installer_process_config_parse_base as cfg;
use compositor_installer_process_layout_compute_base as layout;
use compositor_installer_process_packages_enumerate_base as pkg;

/// Non-interactive: print the default preset's identity + complete settings JSON (one
/// per line, `id<TAB>desktop_name<TAB>json`) for the single Y5 Compositor session. The
/// JSON is built exactly as the installer seeds it (shared schema + defaults). Used by CI.
pub fn emit_presets() {
    let base = cfg::BaseConfig::default();
    let encoder = pkg::capture_encoder_for(pkg::detect_gpu());
    for p in &cfg::default_presets(&base, encoder) {
        println!("{}\t{}\t{}", p.id, p.desktop_name, layout::settings_json(p).replace('\n', " "));
    }
}

/// Non-interactive: print the runtime package names for a manager, one per line, across
/// EVERY group (runtime, xwayland, devtool, diagnostics, toolchain). Used by the pre-CI
/// `verify-packages.sh` gate to resolve names against each distro's base image without a
/// build. `spec` is `<mgr>` or `<mgr>:<release>` (e.g. `apt:12`). Returns false on a bad
/// spec so `main` can exit nonzero.
pub fn emit_packages(spec: &str) -> bool {
    let (mgr_str, release) = match spec.split_once(':') {
        Some((m, r)) => (m, Some(r.to_string())),
        None => (spec, None),
    };
    let Some(mgr) = pkg::PackageManager::parse(mgr_str) else {
        eprintln!("--emit-packages: unknown manager '{mgr_str}' (want dnf|apt|pacman|nix)");
        return false;
    };
    for group in pkg::groups(mgr, release.as_deref()) {
        for p in group.packages {
            println!("{p}");
        }
    }
    true
}

pub fn print_help() {
    println!(
        "y5-install — interactive y5 compositor installer\n\n\
         USAGE:\n  y5-install [--dry-run] [--help]\n\n\
         Run from the unzipped artifact directory (prebuilt binaries + templates\n\
         next to this executable, or set Y5_INSTALL_STAGE).\n\n\
         OPTIONS:\n  \
         -n, --dry-run              Print the install + file actions without changing anything.\n  \
         --emit-packages=<mgr[:rel]> List every runtime package name for a manager\n  \
         \x20                        (dnf|apt|pacman|nix), one per line, and exit. For the\n  \
         \x20                        pre-CI package-name verifier. e.g. --emit-packages=apt:12\n  \
         -h, --help                 Show this help.\n"
    );
}
