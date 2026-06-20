//! Installer steps 2-3: prompt the default Y5 Desktop configuration (values
//! propagate to every preset) and build the preset list (incl. optional Custom).

use compositor_installer_process_config_parse_base as cfg;
use compositor_installer_process_config_parse_base::prompt;

/// Prompt the base configuration, expand it into the standard preset matrix, and
/// optionally append the Custom preset.
pub fn gather() -> Vec<cfg::Preset> {
    println!("\n-- Default Y5 Desktop configuration (propagates to all presets) --");
    let defaults = cfg::BaseConfig::default();
    let base = cfg::BaseConfig {
        render_node: prompt::ask("render_node", "DRM render node path", &defaults.render_node),
        desktop_name_root: prompt::ask(
            "desktop_name",
            "Root XDG desktop name (variants append Dev/Exp/…)",
            &defaults.desktop_name_root,
        ),
        log_level: prompt::ask("log_level", "Developer log levels", &defaults.log_level),
        depth: if prompt::yes_no("depth", "10-bit deep-color scanout (no = 8-bit)", defaults.depth == 10) {
            10
        } else {
            8
        },
        vrr: prompt::yes_no("vrr", "Enable adaptive sync / VRR", defaults.vrr),
        renderer_fallback: prompt::yes_no(
            "renderer_fallback",
            "Fall back to GLES if Vulkan init fails",
            defaults.renderer_fallback,
        ),
        renderer_sync: prompt::choose(
            "renderer_sync",
            "Default frame-sync strategy",
            &["", "infence", "kms"],
            &defaults.renderer_sync,
        ),
    };

    let mut presets = cfg::default_presets(&base);
    // Always offer the Custom entry (Y5CompositorCustom).
    if prompt::yes_no("custom", "Also create a Y5CompositorCustom preset", true) {
        let env = cfg::prompt_custom_env(&base);
        presets.push(cfg::custom_preset(&base.desktop_name_root, env));
    }
    presets
}
