//! Package-group model and best-effort GPU-vendor detection. Pure std.

use std::process::Command;

/// A user-selectable group of dnf packages.
#[derive(Clone, Debug)]
pub struct PackageGroup {
    /// Stable key, e.g. "base", "mesa", "nvidia".
    pub key: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub packages: Vec<&'static str>,
    /// Whether this group is pre-selected by default.
    pub default_on: bool,
}

/// Detected primary GPU vendor (best-effort, from `lspci`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gpu {
    Nvidia,
    Amd,
    Intel,
    Unknown,
}

/// Best-effort GPU vendor detection from `lspci`. Used to pre-select the Mesa vs
/// NVIDIA driver group; never fatal.
pub fn detect_gpu() -> Gpu {
    let out = Command::new("lspci").output();
    let text = match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_lowercase(),
        Err(_) => return Gpu::Unknown,
    };
    // Only consider VGA / 3D / Display controller lines.
    let gpu_lines: String = text
        .lines()
        .filter(|l| l.contains("vga") || l.contains("3d controller") || l.contains("display controller"))
        .collect::<Vec<_>>()
        .join("\n");
    if gpu_lines.contains("nvidia") {
        Gpu::Nvidia
    } else if gpu_lines.contains("amd") || gpu_lines.contains("ati") || gpu_lines.contains("radeon") {
        Gpu::Amd
    } else if gpu_lines.contains("intel") {
        Gpu::Intel
    } else {
        Gpu::Unknown
    }
}
