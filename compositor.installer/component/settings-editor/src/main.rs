//! `y5.compositor.settings` — interactive editor for the compositor's settings
//! file. Writes a COMPLETE `~/.config/y5.compositor/settings.json` (every field;
//! the compositor has no defaults and panics on a missing/partial file).
//!
//!   y5.compositor.settings                  # interactive, default path
//!   y5.compositor.settings --config-file=P  # interactive, path P
//!   y5.compositor.settings --write-default  # non-interactive, write the template

mod edit;
mod prompt;
mod template;

use compositor_developer_environment_config_base::base::{resolve_path, Environment};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{}", USAGE);
        return;
    }
    let write_default = args.iter().any(|a| a == "--write-default");

    // `resolve_path` is the same logic the compositor uses (honors --config-file=,
    // else $XDG_CONFIG_HOME/.config), so the tool and compositor never disagree.
    let path = resolve_path();

    // Pre-fill from an existing file when present; fall back to the template.
    let base = load_existing(&path).unwrap_or_else(template::default_settings);

    let settings = if write_default {
        template::default_settings()
    } else {
        edit::interactive(base)
    };

    let json = serde_json::to_string_pretty(&settings).expect("Environment serializes to JSON");
    match atomic_write(&path, json.as_bytes()) {
        Ok(()) => println!("\nWrote {} ✓", path.display()),
        Err(e) => {
            eprintln!("\nfailed to write {}: {e}", path.display());
            std::process::exit(1);
        }
    }
}

/// Load and parse an existing settings file, or `None` if absent/unreadable/invalid.
fn load_existing(path: &Path) -> Option<Environment> {
    let raw = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&raw) {
        Ok(env) => Some(env),
        Err(e) => {
            eprintln!("note: existing {} is invalid ({e}); starting from defaults.", path.display());
            None
        }
    }
}

/// Write `bytes` to `path` atomically: create parent dirs, write a sibling temp
/// file, then rename over the target so a reader never sees a partial file.
fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)
}

const USAGE: &str = "\
y5.compositor.settings — author ~/.config/y5.compositor/settings.json

USAGE:
    y5.compositor.settings [OPTIONS]

OPTIONS:
    --config-file=<PATH>   Write to PATH instead of the default location.
    --write-default        Non-interactive: write the canonical default settings.
    -h, --help             Show this help.
";
