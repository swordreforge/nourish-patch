//! The standard preset matrix (renderer × experimental × sync).

use compositor_installer_process_config_parse_model::{BaseConfig, Env};
use compositor_installer_process_config_parse_preset::{DEV_BINARY, Preset, SYSTEM_BINARY};

/// Build the standard preset matrix from the prompted base config. Shared values
/// (render node, log level, VRR, fallback) propagate into every preset; the axes
/// that define a preset (renderer / depth / hdr / sync / identity) vary.
pub fn default_presets(base: &BaseConfig) -> Vec<Preset> {
    let root = &base.desktop_name_root;
    let mk = |renderer: &str, sync: &str, hdr: bool, depth: u8, suffix: &str| Env {
        renderer: renderer.to_string(),
        renderer_fallback: base.renderer_fallback,
        renderer_sync: sync.to_string(),
        hdr,
        depth,
        vrr: base.vrr,
        render_node: base.render_node.clone(),
        desktop_name: format!("{root}{suffix}"),
        log_level: base.log_level.clone(),
        vk_diag: String::new(),
        capture_encoder: "nvenc".to_string(),
        window_client_size_fallback: false,
        window_subsurface_shrinks: false,
    };

    vec![
        Preset {
            id: "default".into(),
            label: "Y5 Desktop (default)".into(),
            desktop_name: root.clone(),
            session_name: "Y5".into(),
            wrapper: "y5.compositor.desktop".into(),
            service: "y5.service".into(),
            wayland_session: "y5-compositor.desktop".into(),
            binary: SYSTEM_BINARY.into(),
            env: mk("vulkan", &base.renderer_sync, false, base.depth, ""),
        },
        Preset {
            id: "dev".into(),
            label: "Dev (Vulkan, depth 10)".into(),
            desktop_name: format!("{root}Dev"),
            session_name: "Y5Dev".into(),
            wrapper: "y5.compositor.dev.desktop".into(),
            service: "y5.dev.service".into(),
            wayland_session: "y5-compositor-dev.desktop".into(),
            binary: DEV_BINARY.into(),
            env: mk("vulkan", "infence", false, 10, "Dev"),
        },
        Preset {
            id: "gles".into(),
            label: "Gles".into(),
            desktop_name: format!("{root}DevGles"),
            session_name: "Y5DevGles".into(),
            wrapper: "y5.compositor.dev.gles.desktop".into(),
            service: "y5.dev.gles.service".into(),
            wayland_session: "y5-compositor-dev-gles.desktop".into(),
            binary: DEV_BINARY.into(),
            env: mk("gles", "infence", false, 8, "DevGles"),
        },
        Preset {
            id: "exp".into(),
            label: "Experimental (HDR)".into(),
            desktop_name: format!("{root}Exp"),
            session_name: "Y5Exp".into(),
            wrapper: "y5.compositor.exp.desktop".into(),
            service: "y5.exp.service".into(),
            wayland_session: "y5-compositor-exp.desktop".into(),
            binary: DEV_BINARY.into(),
            env: mk("vulkan", "infence", true, 10, "Exp"),
        },
        Preset {
            id: "nosync".into(),
            label: "Nosync (sync off)".into(),
            desktop_name: format!("{root}Nosync"),
            session_name: "Y5Nosync".into(),
            wrapper: "y5.compositor.nosync.desktop".into(),
            service: "y5.nosync.service".into(),
            wayland_session: "y5-compositor-nosync.desktop".into(),
            binary: DEV_BINARY.into(),
            env: mk("vulkan", "", false, 10, "Nosync"),
        },
        Preset {
            id: "kmssync".into(),
            label: "KMS-sync".into(),
            desktop_name: format!("{root}KmsSync"),
            session_name: "Y5KmsSync".into(),
            wrapper: "y5.compositor.kmssync.desktop".into(),
            service: "y5.kmssync.service".into(),
            wayland_session: "y5-compositor-kmssync.desktop".into(),
            binary: DEV_BINARY.into(),
            env: mk("vulkan", "kms", false, 10, "KmsSync"),
        },
    ]
}
