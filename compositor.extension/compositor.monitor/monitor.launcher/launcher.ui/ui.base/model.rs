//! Data types owned by the launcher.

use std::path::PathBuf;
use std::time::SystemTime;

/// A launchable application.
///
/// The compositor populates a `Vec<Application>` and hands it to the
/// launcher. The launcher takes ownership; it never mutates the
/// compositor's copy.
#[derive(Debug, Clone)]
pub struct Application {
    /// Stable identifier (e.g. desktop file id). Echoed back in
    /// [`crate::message::LauncherMessage::Launch`] so the compositor
    /// can correlate.
    pub id: String,

    /// Human-readable name shown under the focused icon.
    pub title: String,

    /// Executable to invoke.
    pub bin: PathBuf,

    /// Arguments to pass to `bin`.
    pub args: Vec<String>,

    /// Optional path to an icon file (PNG or SVG). If `None` or the
    /// file fails to load, a glyph fallback is drawn.
    pub icon_path: Option<PathBuf>,

    /// How many times the user has launched this app via the launcher.
    pub usage_count: u64,

    /// When the user last launched this app. `None` = never launched
    /// here.
    pub usage_time: Option<SystemTime>,
}

/// Direction the user picked after focusing an icon. Emitted inside
/// [`crate::message::LauncherMessage::Launch`] so a tiling compositor
/// knows where to place the new window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}
