//! The field-by-field interactive flow. Takes the starting values (an existing
//! file if present, else the template) and returns a fully-populated `Environment`.

use crate::prompt::{ask, ask_u8, choose, yes_no};
use compositor_developer_environment_config_base::base::Environment;

pub fn interactive(base: Environment) -> Environment {
    println!("y5.compositor.settings — every field is required; press Enter to keep the shown value.");
    Environment {
        renderer: choose(
            "renderer",
            "Renderer backend.",
            &["vulkan", "gles"],
            &base.renderer,
        ),
        renderer_fallback: yes_no(
            "renderer_fallback",
            "Fall back to GLES if Vulkan initialization fails.",
            base.renderer_fallback,
        ),
        renderer_sync: choose(
            "renderer_sync",
            "Frame-sync strategy.",
            &["", "infence", "kms"],
            &base.renderer_sync,
        ),
        hdr: yes_no("hdr", "Enable HDR output (Vulkan only).", base.hdr),
        depth: ask_u8(
            "depth",
            "Scanout bit depth: 8 (SDR) or 10 (deep color).",
            &[8, 10],
            base.depth,
        ),
        vrr: yes_no("vrr", "Enable adaptive sync / VRR.", base.vrr),
        render_node: ask(
            "render_node",
            "DRM render node path.",
            &base.render_node,
        ),
        desktop_name: ask(
            "desktop_name",
            "XDG desktop name advertised to clients.",
            &base.desktop_name,
        ),
        log_level: ask(
            "log_level",
            "Developer-log level spec, e.g. info,warn,error.",
            &base.log_level,
        ),
        vk_diag: choose(
            "vk_diag",
            "Vulkan diagnostics overlay.",
            &["", "vk", "blit"],
            &base.vk_diag,
        ),
        capture_encoder: choose(
            "capture_encoder",
            "Hardware video-capture encoder (mesa/vaapi -> VAAPI, else NVENC).",
            &["nvenc", "vaapi", "mesa"],
            &base.capture_encoder,
        ),
        window_client_size_fallback: yes_no(
            "window_client_size_fallback",
            "Fall back to client xdg geometry instead of compositor-tracked sizing.",
            base.window_client_size_fallback,
        ),
        window_subsurface_shrinks: yes_no(
            "window_subsurface_shrinks",
            "Fit the whole surface tree so a subsurface can shrink the window.",
            base.window_subsurface_shrinks,
        ),
    }
}
