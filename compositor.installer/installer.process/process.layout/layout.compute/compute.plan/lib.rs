//! Action-plan builders for the binaries, the per-preset session files, and the
//! PAM lock policy.

use std::path::PathBuf;

use compositor_installer_process_config_parse_base::Preset;
use compositor_installer_process_layout_compute_policy as policy;
use compositor_installer_process_layout_compute_session as session;
use compositor_installer_process_layout_compute_stage::{
    Action, Source, Stage, home, user_systemd_dir,
};

/// Actions to install the two compositor binaries (system + dev) from the stage.
pub fn binary_actions(stage: &Stage, presets: &[Preset]) -> Vec<Action> {
    let mut names: Vec<&str> = presets.iter().map(|p| p.binary.as_str()).collect();
    names.sort();
    names.dedup();
    names
        .into_iter()
        .map(|name| Action::Place {
            dest: PathBuf::from("/usr/bin").join(name),
            source: Source::Copy(stage.binary(name)),
            mode: 0o755,
            root: true,
        })
        .collect()
}

/// Actions for a single preset: wrapper script, systemd service + shutdown target,
/// wayland-session entry, and the per-desktop xdg-portal config.
pub fn preset_actions(preset: &Preset) -> Vec<Action> {
    let mut a = Vec::new();

    // /usr/bin wrapper that exports the single COMPOSITOR_ENVIRONMENT + identity.
    a.push(Action::Place {
        dest: PathBuf::from("/usr/bin").join(&preset.wrapper),
        source: Source::Text(session::wrapper_desktop(preset)),
        mode: 0o755,
        root: true,
    });

    // systemd user service + matching shutdown target.
    let sd = user_systemd_dir();
    a.push(Action::Place {
        dest: sd.join(&preset.service),
        source: Source::Text(session::systemd_service(preset)),
        mode: 0o644,
        root: false,
    });
    a.push(Action::Place {
        dest: sd.join(shutdown_target_name(&preset.service)),
        source: Source::Text(session::shutdown_target()),
        mode: 0o644,
        root: false,
    });

    // Display-manager session entry.
    a.push(Action::Place {
        dest: PathBuf::from("/usr/share/wayland-sessions").join(&preset.wayland_session),
        source: Source::Text(session::wayland_session(preset)),
        mode: 0o644,
        root: true,
    });

    // Per-desktop portal config (one per preset — keyed by XDG_CURRENT_DESKTOP).
    a.push(Action::Place {
        dest: home()
            .join(".config/xdg-desktop-portal")
            .join(format!("{}-portals.conf", preset.desktop_name)),
        source: Source::Text(policy::portals_conf()),
        mode: 0o644,
        root: false,
    });

    a
}

/// `y5.dev.service` -> `y5.dev.shutdown.target`.
fn shutdown_target_name(service: &str) -> String {
    format!("{}.shutdown.target", service.strip_suffix(".service").unwrap_or(service))
}

/// Install the PAM lock policy at /etc/pam.d/y5-lock.
pub fn pam_actions(stage: &Stage) -> Vec<Action> {
    let src = stage.template("pam/installation-y5-lock");
    let source = if src.exists() {
        Source::Copy(src)
    } else {
        Source::Text(policy::pam_y5_lock())
    };
    vec![Action::Place {
        dest: PathBuf::from("/etc/pam.d/y5-lock"),
        source,
        mode: 0o644,
        root: true,
    }]
}
