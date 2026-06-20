//! Configuration. A single TOML file at ~/.config/mx-gesture-daemon/config.toml
//! (or wherever you point the daemon).

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Config {
    /// Optional product-name substring to pick a specific device, e.g.
    /// "MX Master 3S". If omitted, the first HID++ device is used.
    #[serde(default)]
    pub device: Option<String>,

    /// Optional hex PID override, e.g. "B034".
    #[serde(default)]
    pub pid: Option<String>,

    /// Movement threshold (in raw mouse counts) before a direction fires.
    /// MX Master accumulates a lot of counts during a gesture; ~80 is a
    /// reasonable starting point. Tune to taste.
    /// Movement threshold (raw counts) before a direction fires.
    #[serde(default = "default_threshold")]
    pub threshold: i32,

    /// Continuous mode: while the button is held, keep firing each time
    /// accumulated movement crosses `threshold`. Default: true.
    #[serde(default = "default_true")]
    pub continuous: bool,

    /// Minimum milliseconds between two consecutive fires in continuous mode.
    /// Acts as a debounce floor so a fast drag can't spam the server.
    #[serde(default = "default_min_interval_ms")]
    pub min_interval_ms: u64,
    /// Whether to also fire a "tap" event when the gesture button is
    /// released without crossing the threshold.
    #[serde(default = "default_true")]
    pub fire_tap: bool,

    /// Action mapping. Each value is a shell command run via `sh -c`.
    /// Recognised keys: up, down, left, right, tap.
    #[serde(default)]
    pub actions: Actions,
}

fn default_min_interval_ms() -> u64 { 80 }

fn default_threshold() -> i32 {
    80
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Actions {
    #[serde(default)]
    pub up: Option<String>,
    #[serde(default)]
    pub down: Option<String>,
    #[serde(default)]
    pub left: Option<String>,
    #[serde(default)]
    pub right: Option<String>,
    #[serde(default)]
    pub tap: Option<String>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        let cfg: Config = toml::from_str(&text)
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(cfg)
    }
}
