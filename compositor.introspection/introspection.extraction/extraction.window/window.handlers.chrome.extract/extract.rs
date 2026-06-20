use compositor_introspection_extraction_window_handlers_chrome_attributes::attributes::{
    BrowserVariant, ProfileDirectory, UserDataDir, Variant,
};
use compositor_introspection_extraction_window_handlers_chrome_attributes_more::attributes::{
    ActiveTabTitleGuess, AppModeUrl, Urls,
};
use compositor_introspection_extraction_window_hints_inferred::inferred::InferredHints;
use compositor_introspection_extraction_window_hints_source::source::{Confidence, SourceMethod};
use compositor_introspection_extraction_window_meta_types::types::Meta;
use std::path::PathBuf;

/// Browser variant from the exe basename.
pub fn push_variant(meta: &Meta, hints: &mut InferredHints) {
    let Some(exe) = &meta.exe else { return };
    let exe_name = exe.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let variant = match exe_name {
        "chrome" | "google-chrome-stable" => BrowserVariant::Chrome,
        "chromium" | "chromium-browser" => BrowserVariant::Chromium,
        "brave" | "brave-browser" => BrowserVariant::Brave,
        "microsoft-edge" => BrowserVariant::Edge,
        "vivaldi" => BrowserVariant::Vivaldi,
        other => BrowserVariant::Other(other.to_string()),
    };
    hints.push::<Variant>(variant, SourceMethod::ProcExe, "parsed from exe basename", Confidence::High);
}

/// Profile dir / user data dir / URLs from cmdline (main process only).
pub fn push_cmdline_hints(meta: &Meta, hints: &mut InferredHints) {
    let Some(cmdline) = &meta.cmdline else { return };
    if cmdline.iter().any(|a| a.starts_with("--type=")) {
        return;
    }
    let mut urls: Vec<String> = Vec::new();
    for arg in cmdline {
        if let Some(v) = arg.strip_prefix("--profile-directory=") {
            hints.push::<ProfileDirectory>(v.to_string(), SourceMethod::ProcCmdline, "--profile-directory flag", Confidence::High);
        }
        if let Some(v) = arg.strip_prefix("--user-data-dir=") {
            hints.push::<UserDataDir>(PathBuf::from(v), SourceMethod::ProcCmdline, "--user-data-dir flag", Confidence::High);
        }
        if let Some(v) = arg.strip_prefix("--app=") {
            hints.push::<AppModeUrl>(v.to_string(), SourceMethod::ProcCmdline, "--app flag", Confidence::High);
        }
        if (arg.starts_with("http://") || arg.starts_with("https://") || arg.starts_with("file://"))
            && !arg.starts_with("--")
        {
            urls.push(arg.clone());
        }
    }
    if !urls.is_empty() {
        hints.push::<Urls>(urls, SourceMethod::ProcCmdline, "positional URL arguments", Confidence::High);
    }
}

/// Active tab title guess from the window title.
pub fn push_title_hint(meta: &Meta, hints: &mut InferredHints) {
    let Some(title) = &meta.title else { return };
    let candidate = title
        .strip_suffix(" - Google Chrome")
        .or_else(|| title.strip_suffix(" - Chromium"))
        .or_else(|| title.strip_suffix(" - Brave"))
        .unwrap_or(title.as_str())
        .trim();
    if !candidate.is_empty() {
        hints.push::<ActiveTabTitleGuess>(candidate.to_string(), SourceMethod::WindowTitle, "stripped browser suffix", Confidence::Medium);
    }
}
