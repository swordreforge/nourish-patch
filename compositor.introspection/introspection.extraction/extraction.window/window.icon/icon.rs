//! Skips the full Icon Theme Specification: no theme inheritance chains, no
//! size threshold matching. Searches `Adwaita` then `hicolor` and falls back
//! to `/usr/share/pixmaps`. Sufficient for v0.

use std::path::PathBuf;

const PREFERRED_THEMES: &[&str] = &["Adwaita", "hicolor"];
const ICON_EXTENSIONS: &[&str] = &["svg", "png", "xpm"];

/// Resolve an icon name (or absolute path) to a file on disk.
pub fn resolve(name: &str) -> Option<PathBuf> {
    if name.starts_with('/') {
        let p = PathBuf::from(name);
        return p.exists().then_some(p);
    }

    for theme in PREFERRED_THEMES {
        if let Some(p) = find_in_theme(name, theme) {
            return Some(p);
        }
    }

    for base in &["/usr/share/pixmaps"] {
        for ext in ICON_EXTENSIONS {
            let candidate = PathBuf::from(base).join(format!("{name}.{ext}"));
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

fn icon_base_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(&home).join(".local/share/icons"));
        dirs.push(PathBuf::from(&home).join(".icons"));
    }
    let system_dirs = std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    for d in system_dirs.split(':') {
        if !d.is_empty() {
            dirs.push(PathBuf::from(d).join("icons"));
        }
    }
    dirs
}

fn find_in_theme(name: &str, theme: &str) -> Option<PathBuf> {
    let bases = icon_base_dirs();
    let size_dirs = [
        "scalable", "512x512", "256x256", "128x128", "96x96", "64x64", "48x48",
        "32x32", "24x24", "22x22", "16x16",
    ];
    let contexts = [
        "apps", "actions", "categories", "places", "mimetypes", "devices", "status",
    ];

    for base in &bases {
        let theme_root = base.join(theme);
        if !theme_root.exists() {
            continue;
        }
        for size in &size_dirs {
            for context in &contexts {
                for ext in ICON_EXTENSIONS {
                    let candidate = theme_root
                        .join(size)
                        .join(context)
                        .join(format!("{name}.{ext}"));
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
            for ext in ICON_EXTENSIONS {
                let flat = theme_root.join(size).join(format!("{name}.{ext}"));
                if flat.exists() {
                    return Some(flat);
                }
            }
        }
    }
    None
}
