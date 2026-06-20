// Tiny JSON file store under ~/.config/y5.compositor.developer/<kind>/<name>.json.
// Backs the viewer's filter presets and saved log dumps. App commands (see main.rs) are
// always enabled in Tauri 2, so no capability entry is needed for these.

use std::fs;
use std::path::PathBuf;

/// ~/.config/y5.compositor.developer (honoring XDG_CONFIG_HOME).
fn base() -> PathBuf {
    let cfg = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| ".".into());
            home.join(".config")
        });
    cfg.join("y5.compositor.developer")
}

/// `<base>/<kind>`, created if missing.
fn dir(kind: &str) -> Result<PathBuf, String> {
    let d = base().join(kind);
    fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

/// Reject path traversal / odd characters in names.
fn sanitize(name: &str) -> Result<String, String> {
    let n = name.trim();
    if n.is_empty() || n.len() > 128 {
        return Err("name must be 1–128 characters".into());
    }
    if n.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' ')) {
        Ok(n.to_string())
    } else {
        Err("name may only contain letters, digits, space, . _ -".into())
    }
}

/// List entry names (without the `.json`), sorted.
pub fn list(kind: &str) -> Result<Vec<String>, String> {
    let d = dir(kind)?;
    let mut out = Vec::new();
    for entry in fs::read_dir(&d).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                out.push(stem.to_string());
            }
        }
    }
    out.sort();
    Ok(out)
}

pub fn save(kind: &str, name: &str, data: &str) -> Result<(), String> {
    let path = dir(kind)?.join(format!("{}.json", sanitize(name)?));
    fs::write(path, data).map_err(|e| e.to_string())
}

pub fn read(kind: &str, name: &str) -> Result<String, String> {
    let path = dir(kind)?.join(format!("{}.json", sanitize(name)?));
    fs::read_to_string(path).map_err(|e| e.to_string())
}

pub fn remove(kind: &str, name: &str) -> Result<(), String> {
    let path = dir(kind)?.join(format!("{}.json", sanitize(name)?));
    fs::remove_file(path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_sanitize() {
        let tmp = std::env::temp_dir().join(format!("y5tooltest_{}", std::process::id()));
        std::env::set_var("XDG_CONFIG_HOME", &tmp);

        save("presets", "errors only", "{\"levels\":[0]}").unwrap();
        assert_eq!(list("presets").unwrap(), vec!["errors only".to_string()]);
        assert_eq!(read("presets", "errors only").unwrap(), "{\"levels\":[0]}");
        remove("presets", "errors only").unwrap();
        assert!(list("presets").unwrap().is_empty());

        // path traversal / bad names are rejected
        assert!(save("dumps", "../evil", "x").is_err());
        assert!(save("dumps", "", "x").is_err());

        let _ = fs::remove_dir_all(&tmp);
    }
}
