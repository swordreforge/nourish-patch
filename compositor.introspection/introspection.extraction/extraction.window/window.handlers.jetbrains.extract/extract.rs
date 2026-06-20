use compositor_introspection_extraction_window_handlers_jetbrains_attributes::attributes::{
    LauncherKind, LauncherKindAttr, Product, ProductAttr, ProjectNameGuess, ProjectPath,
};
use compositor_introspection_extraction_window_hints_inferred::inferred::InferredHints;
use compositor_introspection_extraction_window_hints_source::source::{Confidence, SourceMethod};
use compositor_introspection_extraction_window_meta_types::types::Meta;
use std::path::PathBuf;

/// IDE product parsed from the Wayland app_id.
pub fn push_product(meta: &Meta, hints: &mut InferredHints) {
    let Some(app_id) = &meta.app_id else { return };
    let product = match app_id.as_str() {
        s if s.contains("idea") => Product::IntelliJIDEA,
        s if s.contains("pycharm") => Product::PyCharm,
        s if s.contains("goland") => Product::GoLand,
        s if s.contains("clion") => Product::CLion,
        s if s.contains("webstorm") => Product::WebStorm,
        s if s.contains("rider") => Product::Rider,
        s if s.contains("rubymine") => Product::RubyMine,
        s if s.contains("phpstorm") => Product::PhpStorm,
        s if s.contains("datagrip") => Product::DataGrip,
        other => Product::Unknown(other.to_string()),
    };
    hints.push::<ProductAttr>(product, SourceMethod::WaylandSurface, "parsed from app_id", Confidence::High);
}

/// Launcher kind parsed from the exe path.
pub fn push_launcher(meta: &Meta, hints: &mut InferredHints) {
    let Some(exe) = &meta.exe else { return };
    let s = exe.to_string_lossy();
    let launcher = if s.contains("/JetBrains/Toolbox/") {
        LauncherKind::Toolbox
    } else if s.starts_with("/var/lib/flatpak/")
        || (s.starts_with("/home/") && s.contains("/.local/share/flatpak/"))
    {
        LauncherKind::Flatpak
    } else if s.starts_with("/snap/") {
        LauncherKind::Snap
    } else if s.starts_with("/usr/") || s.starts_with("/opt/") {
        LauncherKind::SystemPackage
    } else {
        LauncherKind::Unknown
    };
    hints.push::<LauncherKindAttr>(launcher, SourceMethod::ProcExe, "parsed from exe path", Confidence::High);
}

/// Project name guessed from the window-title prefix.
pub fn push_title_guess(meta: &Meta, hints: &mut InferredHints) {
    let Some(title) = &meta.title else { return };
    let candidate = title
        .split(" – ")
        .next()
        .and_then(|s| s.split(" - ").next())
        .map(|s| s.split(" [").next().unwrap_or(s))
        .unwrap_or(title.as_str())
        .trim();
    if !candidate.is_empty() && candidate != title {
        hints.push::<ProjectNameGuess>(candidate.to_string(), SourceMethod::WindowTitle, "parsed prefix of window title", Confidence::Medium);
    }
}

/// Project path: last positional cmdline arg that is an existing absolute dir.
pub fn push_project_path(meta: &Meta, hints: &mut InferredHints) {
    let Some(cmdline) = &meta.cmdline else { return };
    for arg in cmdline.iter().rev() {
        if arg.starts_with('-') {
            continue;
        }
        let path = PathBuf::from(arg);
        if path.is_absolute() && path.exists() && path.is_dir() {
            hints.push::<ProjectPath>(path, SourceMethod::ProcCmdline, "positional arg pointing to existing absolute directory", Confidence::High);
            break;
        }
    }
}
