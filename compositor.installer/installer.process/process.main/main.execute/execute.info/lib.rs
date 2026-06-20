//! Non-interactive installer outputs: `--emit-presets` and `--help`.

use compositor_installer_process_config_parse_base as cfg;

/// Non-interactive: print every default preset's identity + COMPOSITOR_ENVIRONMENT
/// JSON (one per line, `id<TAB>desktop_name<TAB>json`). Used for verification/CI.
pub fn emit_presets() {
    let base = cfg::BaseConfig::default();
    let mut presets = cfg::default_presets(&base);
    // A deterministic Custom preset using the base defaults.
    let env = cfg::Env {
        renderer: "vulkan".into(),
        renderer_fallback: base.renderer_fallback,
        renderer_sync: base.renderer_sync.clone(),
        hdr: false,
        depth: base.depth,
        vrr: base.vrr,
        render_node: base.render_node.clone(),
        desktop_name: String::new(),
        log_level: base.log_level.clone(),
        vk_diag: String::new(),
        capture_encoder: "nvenc".into(),
        window_client_size_fallback: false,
        window_subsurface_shrinks: false,
    };
    presets.push(cfg::custom_preset(&base.desktop_name_root, env));
    for p in &presets {
        println!("{}\t{}\t{}", p.id, p.desktop_name, p.env.to_json());
    }
}

pub fn print_help() {
    println!(
        "y5-install — interactive y5 compositor installer\n\n\
         USAGE:\n  y5-install [--dry-run] [--help]\n\n\
         Run from the unzipped artifact directory (prebuilt binaries + templates\n\
         next to this executable, or set Y5_INSTALL_STAGE).\n\n\
         OPTIONS:\n  \
         -n, --dry-run   Print the dnf + file actions without changing anything.\n  \
         -h, --help      Show this help.\n"
    );
}
