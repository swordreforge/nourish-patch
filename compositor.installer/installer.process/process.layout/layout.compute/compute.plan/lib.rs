//! Action-plan builders for the binaries, the per-preset session files, and the
//! PAM lock policy.

use std::path::PathBuf;

use compositor_installer_process_config_parse_base::Preset;
use compositor_installer_process_layout_compute_policy as policy;
use compositor_installer_process_layout_compute_session as session;
use compositor_installer_process_layout_compute_stage::{
    Action, Source, Stage, home, user_systemd_dir,
};

/// `Action::Place` shorthand.
fn place(dest: PathBuf, source: Source, mode: u32, root: bool) -> Action {
    Action::Place { dest, source, mode, root }
}

/// Actions to install the compositor binaries (system + dev) from the stage, plus the
/// `y5.compositor.settings` configuration tool.
pub fn binary_actions(stage: &Stage, presets: &[Preset]) -> Vec<Action> {
    let mut names: Vec<&str> = presets.iter().map(|p| p.binary.as_str()).collect();
    names.sort();
    names.dedup();
    let mut actions: Vec<Action> = names
        .into_iter()
        .map(|name| place(PathBuf::from("/usr/bin").join(name), Source::Copy(stage.binary(name)), 0o755, true))
        .collect();

    // The settings tool authors settings.json (the wrappers no longer write it), so it's
    // a REQUIRED binary — placed unconditionally like the compositor binaries above, so
    // every install refreshes it to the bundled release. A bundle missing it is broken
    // and the strict install rightly fails rather than silently shipping a stale tool.
    actions.push(place(
        PathBuf::from("/usr/bin/y5.compositor.settings"),
        Source::Copy(stage.binary("y5.compositor.settings")),
        0o755,
        true,
    ));
    actions
}

/// Seed the compositor's settings.json in the user config dir from the prompted config.
/// The session wrappers no longer write it, so the installer authors it here (and edit
/// it later with `y5.compositor.settings`). root=false → lands in the invoking user's
/// $HOME/.config, which is why the installer must run unprivileged.
pub fn settings_action(preset: &Preset) -> Action {
    place(
        home().join(".config/y5.compositor/settings.json"),
        Source::Text(preset.env.to_json()),
        0o644,
        false,
    )
}

/// Actions for a single preset: wrapper script, systemd service + shutdown target,
/// wayland-session entry, and the per-desktop xdg-portal config.
pub fn preset_actions(preset: &Preset) -> Vec<Action> {
    let sd = user_systemd_dir();
    let portal = home()
        .join(".config/xdg-desktop-portal")
        .join(format!("{}-portals.conf", preset.desktop_name));
    vec![
        // /usr/bin wrapper: exports identity, checks settings.json exists (never writes
        // it — see session::wrapper_desktop), then execs the compositor.
        place(PathBuf::from("/usr/bin").join(&preset.wrapper), Source::Text(session::wrapper_desktop(preset)), 0o755, true),
        // systemd user service + matching shutdown target.
        place(sd.join(&preset.service), Source::Text(session::systemd_service(preset)), 0o644, false),
        place(sd.join(shutdown_target_name(&preset.service)), Source::Text(session::shutdown_target()), 0o644, false),
        // Display-manager session entry.
        place(PathBuf::from("/usr/share/wayland-sessions").join(&preset.wayland_session), Source::Text(session::wayland_session(preset)), 0o644, true),
        // Per-desktop portal config (keyed by XDG_CURRENT_DESKTOP).
        place(portal, Source::Text(policy::portals_conf()), 0o644, false),
    ]
}

/// `y5.dev.service` -> `y5.dev.shutdown.target`.
fn shutdown_target_name(service: &str) -> String {
    format!("{}.shutdown.target", service.strip_suffix(".service").unwrap_or(service))
}

/// Install the PAM lock policy at /etc/pam.d/y5-lock.
pub fn pam_actions(stage: &Stage) -> Vec<Action> {
    let src = stage.template("pam/installation-y5-lock");
    let source = if src.exists() { Source::Copy(src) } else { Source::Text(policy::pam_y5_lock()) };
    vec![place(PathBuf::from("/etc/pam.d/y5-lock"), source, 0o644, true)]
}
