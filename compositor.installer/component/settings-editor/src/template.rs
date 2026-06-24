//! The canonical complete starting settings. The compositor itself has NO
//! defaults (every field is required, it panics on a missing file), so this tool
//! owns the one source of starting values. Every field is set — the produced
//! `settings.json` is always fully populated.

use compositor_developer_environment_config_base::base::Environment;

pub fn default_settings() -> Environment {
    Environment {
        renderer: "vulkan".to_string(),
        renderer_fallback: true,
        renderer_sync: String::new(),
        hdr: false,
        depth: 8,
        vrr: false,
        render_node: "/dev/dri/renderD128".to_string(),
        desktop_name: "Y5Compositor".to_string(),
        log_level: "info,warn,error".to_string(),
        vk_diag: String::new(),
        capture_encoder: "nvenc".to_string(),
        capture_codec: "av1".to_string(),
        capture_quality: "lossless".to_string(),
        capture_refresh_rate_max: 60,
        capture_background_encoder: String::new(),
        window_client_size_fallback: false,
        window_subsurface_shrinks: false,
    }
}
