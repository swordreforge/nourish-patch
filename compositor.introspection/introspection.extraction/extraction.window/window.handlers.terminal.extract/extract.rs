use compositor_introspection_extraction_window_handlers_terminal_attributes::attributes::{
    ForegroundCwd, LaunchCwd, Shell, TerminalKind, TerminalKindAttr,
};
use compositor_introspection_extraction_window_hints_inferred::inferred::InferredHints;
use compositor_introspection_extraction_window_hints_source::source::{Confidence, SourceMethod};
use compositor_introspection_extraction_window_meta_types::types::{Meta, MetaNode};

/// Map a Wayland app_id to a known terminal kind.
pub fn terminal_kind_from_app_id(app_id: &str) -> TerminalKind {
    match app_id {
        "Alacritty" | "alacritty" => TerminalKind::Alacritty,
        "foot" => TerminalKind::Foot,
        "org.gnome.Terminal" => TerminalKind::GnomeTerminal,
        "org.gnome.Console" | "kgx" => TerminalKind::GnomeConsole,
        "org.gnome.Ptyxis" => TerminalKind::Ptyxis,
        "kitty" => TerminalKind::Kitty,
        "org.wezfurlong.wezterm" => TerminalKind::WezTerm,
        "org.kde.konsole" => TerminalKind::Konsole,
        s if s.starts_with("xterm") => TerminalKind::Xterm,
        other => TerminalKind::Unknown(other.to_string()),
    }
}

/// Terminal kind from app_id.
pub fn push_kind(meta: &Meta, hints: &mut InferredHints) {
    let Some(app_id) = &meta.app_id else { return };
    let kind = terminal_kind_from_app_id(app_id);
    let conf = if matches!(kind, TerminalKind::Unknown(_)) {
        Confidence::Low
    } else {
        Confidence::High
    };
    hints.push::<TerminalKindAttr>(kind, SourceMethod::WaylandSurface, "matched against known terminal app_ids", conf);
}

/// The terminal process's own cwd. Often the launch directory, not where
/// the user is now — low confidence.
pub fn push_launch_cwd(meta: &Meta, hints: &mut InferredHints) {
    if let Some(cwd) = &meta.cwd {
        hints.push::<LaunchCwd>(cwd.clone(), SourceMethod::ProcExe, "terminal's own cwd (launch directory)", Confidence::Low);
    }
}

/// Look at children for a shell, and use the shell's cwd as a much better
/// signal for "where the user is now."
pub fn push_shell_children(node: &MetaNode, hints: &mut InferredHints) {
    for child in &node.children {
        if let Some(comm) = &child.meta.comm {
            if matches!(comm.as_str(), "bash" | "zsh" | "fish" | "sh") {
                hints.push::<Shell>(
                    comm.clone(),
                    SourceMethod::ProcTree,
                    format!("shell child pid={}", child.meta.pid.unwrap_or(0)),
                    Confidence::High,
                );
                if let Some(cwd) = &child.meta.cwd {
                    hints.push::<ForegroundCwd>(
                        cwd.clone(),
                        SourceMethod::ProcTree,
                        format!("shell child pid={} cwd", child.meta.pid.unwrap_or(0)),
                        Confidence::High,
                    );
                }
            }
        }
    }
}
