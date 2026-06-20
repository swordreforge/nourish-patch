//! Action-plan builders for the optional components: MX gesture daemon, polkit
//! agent, and the developer tool window.

use std::path::PathBuf;

use compositor_installer_process_layout_compute_stage::{
    Action, Source, Stage, home, user_systemd_dir,
};

/// Install + enable the MX gesture daemon (binary, udev rule, config, user service).
pub fn mx_actions(stage: &Stage) -> Vec<Action> {
    let mut a = vec![
        Action::Place {
            dest: home().join(".local/bin/mx-gesture-daemon"),
            source: Source::Copy(stage.binary("mx-gesture-daemon")),
            mode: 0o755,
            root: false,
        },
        Action::Place {
            dest: PathBuf::from("/etc/udev/rules.d/42-logitech-hidpp.rules"),
            source: Source::Copy(stage.template("mx/42-logitech-hidpp.rules")),
            mode: 0o644,
            root: true,
        },
        Action::Place {
            dest: home().join(".config/mx-gesture-daemon/config.toml"),
            source: Source::Copy(stage.template("mx/config.example.toml")),
            mode: 0o644,
            root: false,
        },
        Action::Place {
            dest: user_systemd_dir().join("mx-gesture-daemon.service"),
            source: Source::Copy(stage.template("mx/mx-gesture-daemon.service")),
            mode: 0o644,
            root: false,
        },
        Action::UdevReload,
    ];
    a.push(Action::SystemctlUser(vec!["daemon-reload".into()]));
    a.push(Action::SystemctlUser(vec!["enable".into(), "mx-gesture-daemon.service".into()]));
    a
}

/// Install + enable the polkit agent (binary + a new user systemd service).
pub fn polkit_actions(stage: &Stage) -> Vec<Action> {
    vec![
        Action::Place {
            dest: PathBuf::from("/usr/local/bin/y5-polkit-agent"),
            source: Source::Copy(stage.binary("y5-polkit-agent")),
            mode: 0o755,
            root: true,
        },
        Action::Place {
            dest: user_systemd_dir().join("y5-polkit-agent.service"),
            source: Source::Text(
                compositor_installer_process_layout_compute_policy::polkit_service(),
            ),
            mode: 0o644,
            root: false,
        },
        Action::SystemctlUser(vec!["daemon-reload".into()]),
        Action::SystemctlUser(vec!["enable".into(), "y5-polkit-agent.service".into()]),
    ]
}

/// Install the developer tool window binary.
pub fn devtool_actions(stage: &Stage) -> Vec<Action> {
    vec![Action::Place {
        dest: PathBuf::from("/usr/local/bin/compositor-developer-tool"),
        source: Source::Copy(stage.binary("compositor-developer-tool")),
        mode: 0o755,
        root: true,
    }]
}

/// Install + enable the patched xwayland-satellite (X11-app compatibility): the
/// binary at the path its service expects, plus the user systemd service. Pairs
/// with the default-on `xwayland` package group (the `xorg-x11-server-Xwayland`
/// the satellite drives).
pub fn xwayland_actions(stage: &Stage) -> Vec<Action> {
    vec![
        Action::Place {
            dest: PathBuf::from("/usr/bin/xwayland-satellite"),
            source: Source::Copy(stage.binary("xwayland-satellite")),
            mode: 0o755,
            root: true,
        },
        Action::Place {
            dest: user_systemd_dir().join("xwayland.service"),
            source: Source::Copy(stage.template("xwayland/xwayland.service")),
            mode: 0o644,
            root: false,
        },
        Action::SystemctlUser(vec!["daemon-reload".into()]),
        Action::SystemctlUser(vec!["enable".into(), "xwayland.service".into()]),
    ]
}
