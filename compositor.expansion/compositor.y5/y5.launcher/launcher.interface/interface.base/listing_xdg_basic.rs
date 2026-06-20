//! Minimal XDG `.desktop` -> `Application` loader.
//!
//! Scans `$XDG_DATA_DIRS` (plus `$XDG_DATA_HOME`) for `applications/*.desktop`,
//! filters to ones that are actually launchable (have an `Exec`, aren't
//! `NoDisplay=true`, aren't `Hidden=true`, current desktop isn't excluded),
//! resolves icon names against the icon search dirs, and returns
//! `Vec<Application>` ready to hand to `Launcher::new`.
//!
//! No external dependencies. Hand-rolled `.desktop` parser since the
//! format is simple INI with `=`, locale suffixes in brackets on keys.
//!
//! Notes:
//! - `usage_count` / `usage_time` are zeroed. Persist your own stats
//!   keyed by `id` and merge them in before passing to `Launcher::new`.
//! - `Exec` field codes (`%f %F %u %U %i %c %k`) are stripped per the
//!   XDG spec — they're for opening files, which the launcher doesn't do.
//! - Icon resolution prefers SVG, then the highest-resolution PNG
//!   available. Downscaling a 512×512 source to a 64×64 cell with
//!   linear filtering produces clean results; *up*scaling a 48×48
//!   source to 128×128 physical pixels (HiDPI) is the textbook blurry
//!   icon failure mode. So when in doubt, take the bigger source.
//!   This doesn't honour the user's icon theme cascade (would require
//!   parsing `index.theme`). Good enough for a launcher; swap in
//!   `freedesktop-icons` for perfection.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use compositor_monitor_launcher_ui_base::Application;

/// Load every visible application from the standard XDG locations.
pub fn load_applications() -> Vec<Application> {
    let locale = env::var("LC_MESSAGES")
        .or_else(|_| env::var("LANG"))
        .ok()
        .map(strip_encoding);
    let current_desktops = env::var("XDG_CURRENT_DESKTOP")
        .ok()
        .map(|s| s.split(':').map(|s| s.to_string()).collect::<Vec<_>>())
        .unwrap_or_default();

    let icon_dirs = icon_search_dirs();

    // First-write-wins: $XDG_DATA_HOME entries shadow /usr/share ones
    // with the same desktop-file id.
    let mut by_id: HashMap<String, Application> = HashMap::new();

    for dir in applications_dirs() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("desktop") {
                continue;
            }
            let Some(app) = parse_desktop_file(
                &path,
                locale.as_deref(),
                &current_desktops,
                &icon_dirs,
            ) else {
                continue;
            };
            by_id.entry(app.id.clone()).or_insert(app);
        }
    }

    by_id.into_values().collect()
}

// --- Directory discovery ------------------------------------------------

fn applications_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    let data_home = env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".local/share"))
        });
    if let Some(d) = data_home {
        dirs.push(d.join("applications"));
    }

    let data_dirs = env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    for d in data_dirs.split(':').filter(|s| !s.is_empty()) {
        dirs.push(PathBuf::from(d).join("applications"));
    }

    dirs
}

fn icon_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    let data_home = env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".local/share"))
        });
    if let Some(d) = data_home {
        dirs.push(d.join("icons"));
    }
    if let Ok(home) = env::var("HOME") {
        dirs.push(PathBuf::from(&home).join(".icons"));
    }

    let data_dirs = env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    for d in data_dirs.split(':').filter(|s| !s.is_empty()) {
        dirs.push(PathBuf::from(d).join("icons"));
    }

    dirs.push(PathBuf::from("/usr/share/pixmaps"));

    dirs
}

// --- Desktop file parsing -----------------------------------------------

fn parse_desktop_file(
    path: &Path,
    locale: Option<&str>,
    current_desktops: &[String],
    icon_dirs: &[PathBuf],
) -> Option<Application> {
    let contents = fs::read_to_string(path).ok()?;
    let entry = read_desktop_entry_group(&contents)?;

    if entry.get("Type").map(String::as_str) != Some("Application") {
        return None;
    }
    if entry.get("NoDisplay").map(String::as_str) == Some("true") {
        return None;
    }
    if entry.get("Hidden").map(String::as_str) == Some("true") {
        return None;
    }
    let exec_raw = entry.get("Exec")?;

    if let Some(list) = entry.get("OnlyShowIn") {
        if !desktop_list_matches(list, current_desktops) {
            return None;
        }
    }
    if let Some(list) = entry.get("NotShowIn") {
        if desktop_list_matches(list, current_desktops) {
            return None;
        }
    }

    let name = lookup_localised(&entry, "Name", locale)?;
    if name.is_empty() {
        return None;
    }

    let (bin, args) = parse_exec(exec_raw)?;

    let icon_path = entry
        .get("Icon")
        .and_then(|name| resolve_icon(name, icon_dirs));

    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    Some(Application {
        id,
        title: name,
        bin,
        args,
        icon_path,
        usage_count: 0,
        usage_time: None,
    })
}

fn read_desktop_entry_group(contents: &str) -> Option<HashMap<String, String>> {
    let mut map = HashMap::new();
    let mut in_main = false;
    for raw in contents.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            if in_main {
                break;
            }
            in_main = line == "[Desktop Entry]";
            continue;
        }
        if !in_main {
            continue;
        }
        let (k, v) = line.split_once('=')?;
        map.insert(k.trim().to_string(), v.trim().to_string());
    }
    if map.is_empty() {
        None
    } else {
        Some(map)
    }
}

fn lookup_localised(
    entry: &HashMap<String, String>,
    key: &str,
    locale: Option<&str>,
) -> Option<String> {
    if let Some(loc) = locale {
        let full = format!("{key}[{loc}]");
        if let Some(v) = entry.get(&full) {
            return Some(v.clone());
        }
        if let Some(lang) = loc.split('_').next() {
            let short = format!("{key}[{lang}]");
            if let Some(v) = entry.get(&short) {
                return Some(v.clone());
            }
        }
    }
    entry.get(key).cloned()
}

fn strip_encoding(locale: String) -> String {
    locale.split('.').next().unwrap_or("").to_string()
}

fn desktop_list_matches(list: &str, current_desktops: &[String]) -> bool {
    list.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .any(|d| current_desktops.iter().any(|c| c == d))
}

// --- Exec parsing -------------------------------------------------------

fn parse_exec(exec: &str) -> Option<(PathBuf, Vec<String>)> {
    let mut tokens = exec_tokenise(exec)
        .into_iter()
        .filter_map(|tok| {
            let stripped = strip_field_codes(&tok);
            if stripped.is_empty() {
                None
            } else {
                Some(stripped)
            }
        });
    let bin = tokens.next()?;
    let args: Vec<String> = tokens.collect();
    Some((PathBuf::from(bin), args))
}

fn exec_tokenise(exec: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quote = false;
    let mut chars = exec.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => in_quote = !in_quote,
            '\\' if in_quote => {
                if let Some(&next) = chars.peek() {
                    if matches!(next, '"' | '\\' | '$' | '`') {
                        chars.next();
                        cur.push(next);
                        continue;
                    }
                }
                cur.push(c);
            }
            c if c.is_whitespace() && !in_quote => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn strip_field_codes(token: &str) -> String {
    let mut out = String::with_capacity(token.len());
    let mut chars = token.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('%') => out.push('%'),
            Some(_) => {}
            None => {}
        }
    }
    out
}

// --- Icon resolution ----------------------------------------------------

/// Themes searched in priority order. `Adwaita` matches what GTK
/// applications typically render. `hicolor` is the freedesktop-spec
/// default and is required to exist (most apps install their icons
/// there directly).
const PREFERRED_THEMES: &[&str] = &["Adwaita", "hicolor"];

/// File extensions tried at each candidate path, in priority order.
/// SVG first → vector graphics scale cleanly at any render size.
const ICON_EXTENSIONS: &[&str] = &["svg", "svgz", "png", "xpm"];

/// Hicolor-style size directories in **descending** order of pixel
/// count. The first match wins, so the resolver naturally picks the
/// biggest source it can find — exactly what we want, since
/// downscaling a 512px source to the launcher's render size looks
/// clean while upscaling a 48px source to HiDPI is the textbook
/// blurry-icon failure.
///
/// `scalable/` first → vectors. `@2` HiDPI variants (e.g.
/// `256x256@2` holds physically 512-pixel art) are placed above their
/// plain counterparts so they win when present.
const SIZE_DIRS: &[&str] = &[
    "scalable",
    "1024x1024",
    "512x512@2",
    "512x512",
    "256x256@2",
    "256x256",
    "192x192",
    "128x128@2",
    "128x128",
    "96x96@2",
    "96x96",
    "64x64@2",
    "64x64",
    "48x48@2",
    "48x48",
    "32x32@2",
    "32x32",
    "24x24@2",
    "24x24",
    "22x22",
    "16x16@2",
    "16x16",
];

/// Hicolor context subdirectories. Apps are by far the most common;
/// the others are listed so we still find e.g. mimetype icons used as
/// fallbacks. Order doesn't affect quality (we've already locked in
/// the size by the time we hit this loop).
const ICON_CONTEXTS: &[&str] = &[
    "apps",
    "actions",
    "categories",
    "places",
    "mimetypes",
    "devices",
    "status",
];

/// Resolve an `Icon=` value into an absolute path on disk.
///
/// Walks each base dir in `icon_dirs` under each preferred theme, in
/// descending size order, returning the first existing match. This is
/// a simplified version of the freedesktop Icon Theme Specification
/// lookup — no theme inheritance, no exact-size-match preference. We
/// always take the biggest available source and let the renderer
/// downscale, because downscaling produces clean results while
/// upscaling to a HiDPI surface produces blur.
///
/// Falls back to flat directories (`/usr/share/pixmaps`-style) for
/// the long tail of apps that don't install into a theme tree.
fn resolve_icon(name: &str, icon_dirs: &[PathBuf]) -> Option<PathBuf> {
    let p = Path::new(name);
    if p.is_absolute() {
        return p.exists().then(|| p.to_path_buf());
    }

    // 1. Theme-tree lookup: <base>/<theme>/<size>/<context>/<name>.<ext>
    for theme in PREFERRED_THEMES {
        for base in icon_dirs {
            let theme_root = base.join(theme);
            if !theme_root.is_dir() {
                continue;
            }
            if let Some(p) = find_in_theme_tree(&theme_root, name) {
                return Some(p);
            }
        }
    }

    // 2. Flat fallback: <base>/<name>.<ext>
    //    Used by /usr/share/pixmaps and legacy installs.
    for base in icon_dirs {
        for ext in ICON_EXTENSIONS {
            let candidate = base.join(format!("{name}.{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

/// Search a single theme root (e.g. `/usr/share/icons/hicolor/`) for
/// `name`, preferring larger size dirs over smaller ones.
fn find_in_theme_tree(theme_root: &Path, name: &str) -> Option<PathBuf> {
    for size in SIZE_DIRS {
        let size_dir = theme_root.join(size);
        if !size_dir.is_dir() {
            continue;
        }

        // Try every context subdir at this size.
        for context in ICON_CONTEXTS {
            for ext in ICON_EXTENSIONS {
                let candidate = size_dir.join(context).join(format!("{name}.{ext}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }

        // Some themes drop the context layer and put files directly
        // under the size dir (e.g. <theme>/48x48/foo.png).
        for ext in ICON_EXTENSIONS {
            let flat = size_dir.join(format!("{name}.{ext}"));
            if flat.is_file() {
                return Some(flat);
            }
        }
    }
    None
}