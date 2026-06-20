use compositor_introspection_extraction_window_desktop_entry::desktop::{parse, DesktopEntry};
use std::fs;
use std::path::{Path, PathBuf};

/// XDG search path for desktop entries.
pub fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(home_data) = std::env::var("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(home_data).join("applications"));
    } else if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }
    let system_dirs = std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    for d in system_dirs.split(':') {
        if !d.is_empty() {
            dirs.push(PathBuf::from(d).join("applications"));
        }
    }
    dirs
}

/// Walk all XDG dirs and return every parseable `.desktop` file
/// (excluding `Hidden=true`).
pub fn scan_all() -> Vec<DesktopEntry> {
    let mut out = Vec::new();
    for dir in search_dirs() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                if let Some(de) = parse(&path) {
                    if !de.hidden {
                        out.push(de);
                    }
                }
            }
        }
    }
    out
}

/// Find a desktop entry matching a Wayland app_id. Priority: filename stem
/// exact, StartupWMClass exact, filename stem case-insensitive.
pub fn find_by_app_id(app_id: &str) -> Option<DesktopEntry> {
    let entries = scan_all();

    for de in &entries {
        if let Some(stem) = de.path.file_stem().and_then(|s| s.to_str()) {
            if stem == app_id {
                return Some(de.clone());
            }
        }
    }
    for de in &entries {
        if de.startup_wm_class.as_deref() == Some(app_id) {
            return Some(de.clone());
        }
    }
    for de in &entries {
        if let Some(stem) = de.path.file_stem().and_then(|s| s.to_str()) {
            if stem.eq_ignore_ascii_case(app_id) {
                return Some(de.clone());
            }
        }
    }
    None
}

/// Find a desktop entry by absolute path
/// (for `GIO_LAUNCHED_DESKTOP_FILE` and similar env vars).
pub fn find_by_path(path: &Path) -> Option<DesktopEntry> {
    parse(path)
}
