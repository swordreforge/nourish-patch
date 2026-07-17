use std::collections::HashMap;
use std::process::Command;

/// Name advertised to portals — must match `DesktopNames=` in the .desktop
/// and `XDG_CURRENT_DESKTOP` in the wrapper.
// const DESKTOP_NAME: &str = "Y5Compositor";

/// Propagate the session environment to systemd --user and the D-Bus
/// activation environment. Call ONCE, after the Wayland socket is listening
/// and all backends are initialized.
///
/// `wayland_socket` is the name Smithay gave you, e.g. from
/// `ListeningSocketSource::socket_name()` — typically "wayland-1".
///
/// `extra_env` comes from `preferences.json` `env` and is merged on top of
/// the built-in session vars (WAYLAND_DISPLAY, XDG_SESSION_TYPE, etc.).
///
/// Only call this when running as the actual session compositor. Do NOT call
/// it when running nested/embedded for development: it mutates the user-wide
/// systemd and D-Bus environment and would clobber the host session's vars.
pub fn announce_session(
    wayland_socket: &str,
    desktop_name: &str,
    extra_env: &HashMap<String, String>,
) {
    // Build the argument list: built-in session vars first, then user overrides.
    let mut args: Vec<String> = vec![
        "--systemd".into(),
        format!("WAYLAND_DISPLAY={wayland_socket}"),
        format!("XDG_CURRENT_DESKTOP={desktop_name}"),
        "XDG_SESSION_TYPE=wayland".into(),
    ];
    // User-configured env vars (e.g. DISPLAY=:12, GTK_IM_MODULE=fcitx).
    for (k, v) in extra_env {
        args.push(format!("{k}={v}"));
    }

    let result = Command::new("dbus-update-activation-environment")
        .args(&args)
        .status();

    match result {
        Ok(s) if s.success() => {
            info!("session environment propagated to systemd and D-Bus");
        }
        Ok(s) => {
            warn!("dbus-update-activation-environment exited with status {s}");
        }
        Err(e) => {
            warn!(
                "could not run dbus-update-activation-environment: {e} \
                        (is it on PATH? it ships with dbus)"
            );
        }
    }
}
