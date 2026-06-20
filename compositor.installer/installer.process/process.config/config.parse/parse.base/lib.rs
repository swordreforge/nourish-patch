//! Installer configuration model: the per-preset compositor environment, the
//! preset matrix (renderer × experimental × sync, plus Custom), JSON emission for
//! the single `COMPOSITOR_ENVIRONMENT` variable, and interactive prompt helpers.
//!
//! Façade: the implementation lives in the sibling `parse.*` crates; this crate
//! re-exports the original public surface unchanged. Pure std.

/// Minimal interactive stdin prompts (no external TUI dependency).
pub mod prompt {
    pub use compositor_installer_process_config_parse_prompt::*;
}

pub use compositor_installer_process_config_parse_custom::prompt_custom_env;
pub use compositor_installer_process_config_parse_matrix::default_presets;
pub use compositor_installer_process_config_parse_model::{BaseConfig, Env};
pub use compositor_installer_process_config_parse_preset::{Preset, custom_preset};
