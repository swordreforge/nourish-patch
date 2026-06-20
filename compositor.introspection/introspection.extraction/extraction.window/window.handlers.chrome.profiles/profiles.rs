use compositor_introspection_extraction_window_handlers_chrome_attributes::attributes::{
    AvailableProfiles, BrowserVariant, ChromeProfileInfo, UserDataDir, Variant,
};
use compositor_introspection_extraction_window_hints_inferred::inferred::InferredHints;
use compositor_introspection_extraction_window_hints_source::source::{Confidence, SourceMethod};
use std::path::{Path, PathBuf};

/// Default user-data dir for a known browser variant.
pub fn default_user_data_dir(variant: Option<BrowserVariant>) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let suffix = match variant? {
        BrowserVariant::Chrome => ".config/google-chrome",
        BrowserVariant::Chromium => ".config/chromium",
        BrowserVariant::Brave => ".config/BraveSoftware/Brave-Browser",
        BrowserVariant::Edge => ".config/microsoft-edge",
        BrowserVariant::Vivaldi => ".config/vivaldi",
        BrowserVariant::Other(_) => return None,
    };
    Some(PathBuf::from(home).join(suffix))
}

/// Subdirectories of the user-data dir that look like profiles.
pub fn enumerate_chrome_profile_dirs(user_data_dir: &Path) -> Vec<ChromeProfileInfo> {
    let Ok(rd) = std::fs::read_dir(user_data_dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in rd.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !path.join("Preferences").exists() {
            continue;
        }
        if name == "Default" || name.starts_with("Profile ") {
            out.push(ChromeProfileInfo {
                directory_name: name.to_string(),
                display_name: name.to_string(),
            });
        }
    }
    out.sort_by(|a, b| match (a.directory_name.as_str(), b.directory_name.as_str()) {
        ("Default", "Default") => std::cmp::Ordering::Equal,
        ("Default", _) => std::cmp::Ordering::Less,
        (_, "Default") => std::cmp::Ordering::Greater,
        (a, b) => a.cmp(b),
    });
    out
}

/// Enumerate profile directories from the best-known user-data dir.
pub fn push_available_profiles(hints: &mut InferredHints) {
    let user_data_dir = hints
        .best_value::<UserDataDir>()
        .or_else(|| default_user_data_dir(hints.best_value::<Variant>()));
    if let Some(dir) = user_data_dir {
        let profiles = enumerate_chrome_profile_dirs(&dir);
        if !profiles.is_empty() {
            hints.push::<AvailableProfiles>(profiles, SourceMethod::DerivedFromConfig, "Chrome user-data-dir subdirectories", Confidence::High);
        }
    }
}
