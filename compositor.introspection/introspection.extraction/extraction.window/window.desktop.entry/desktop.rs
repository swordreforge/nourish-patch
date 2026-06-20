//! Pure file I/O; no dependency on hint or plan types.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DesktopEntry {
    pub path: PathBuf,
    pub name: String,
    pub icon: Option<String>,
    pub exec: Option<String>,
    pub dbus_activatable: bool,
    pub startup_wm_class: Option<String>,
    pub no_display: bool,
    pub hidden: bool,
}

/// Parse a single .desktop file's `[Desktop Entry]` section. Localized keys
/// (`Name[de]`) and `[Desktop Action ...]` subsections are ignored.
pub fn parse(path: &Path) -> Option<DesktopEntry> {
    let contents = fs::read_to_string(path).ok()?;
    let mut current_section: Option<String> = None;
    let mut entries: HashMap<String, String> = HashMap::new();

    for raw in contents.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            current_section = Some(rest.to_string());
            continue;
        }
        if current_section.as_deref() != Some("Desktop Entry") {
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim();
            if key.contains('[') {
                continue;
            }
            let value = line[eq + 1..].trim();
            entries.insert(key.to_string(), unescape(value));
        }
    }

    let name = entries.get("Name").cloned()?;
    Some(DesktopEntry {
        path: path.to_path_buf(),
        name,
        icon: entries.get("Icon").cloned(),
        exec: entries.get("Exec").cloned(),
        dbus_activatable: entries
            .get("DBusActivatable")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false),
        startup_wm_class: entries.get("StartupWMClass").cloned(),
        no_display: entries
            .get("NoDisplay")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false),
        hidden: entries
            .get("Hidden")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false),
    })
}

/// Limited escape decoding per the freedesktop spec.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('s') => out.push(' '),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}
