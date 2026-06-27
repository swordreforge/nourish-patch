//! The System tab: editable `Environment` fields (settings.json). Each edit emits
//! the full edited struct (`SettingsMessage::Env`) — the handler writes
//! settings.json and flags the reboot banner. Each row has a Reset to its default
//! (`default_settings()`). Bools toggle, enums cycle, free strings use a field.
use compositor_developer_environment_config_base::base::{default_settings, Environment};
use compositor_configurator_hardware_gpu_base::base::RenderDevice;
use compositor_support_iced_core_engine_base::Renderer;
use compositor_configurator_settings_surface_message::message::SettingsMessage;
use compositor_configurator_settings_surface_style::style;
use iced_core::{Element, Length, Theme};
use iced_widget::{button, row, scrollable, text, text_input, Column};

type El<'a> = Element<'a, SettingsMessage, Theme, Renderer>;

fn field_row<'a>(label: &str, value: String, edit: SettingsMessage, reset: SettingsMessage) -> El<'a> {
    row![
        button(text(format!("{label}: {value}"))).width(Length::Fill).on_press(edit).style(style::action),
        button(text("Reset")).on_press(reset).style(style::action),
    ].spacing(6).into()
}

fn boolean<'a>(label: &str, e: &Environment, v: bool, def: bool, set: fn(&mut Environment, bool)) -> El<'a> {
    let mut on = e.clone();
    set(&mut on, !v);
    let mut rd = e.clone();
    set(&mut rd, def);
    field_row(label, if v { "on" } else { "off" }.into(), SettingsMessage::Env(on), SettingsMessage::Env(rd))
}

fn cycle<'a>(label: &str, e: &Environment, cur: &str, def: &str, opts: &[&str], set: fn(&mut Environment, String)) -> El<'a> {
    let i = opts.iter().position(|o| *o == cur).unwrap_or(0);
    let mut nx = e.clone();
    set(&mut nx, opts[(i + 1) % opts.len()].to_string());
    let mut rd = e.clone();
    set(&mut rd, def.to_string());
    let shown = if cur.is_empty() { "(off)".to_string() } else { cur.to_string() };
    field_row(label, shown, SettingsMessage::Env(nx), SettingsMessage::Env(rd))
}

fn textfield<'a>(label: &'a str, e: &'a Environment, val: &'a str, def: &str, set: fn(&mut Environment, String)) -> El<'a> {
    let e2 = e.clone();
    let mut rd = e.clone();
    set(&mut rd, def.to_string());
    row![
        text(format!("{label}:")),
        text_input(label, val).width(Length::Fill).on_input(move |s| {
            let mut e = e2.clone();
            set(&mut e, s);
            SettingsMessage::Env(e)
        }),
        button(text("Reset")).on_press(SettingsMessage::Env(rd)).style(style::action),
    ].spacing(6).into()
}

pub fn build<'a>(e: &'a Environment, devices: &'a [RenderDevice]) -> El<'a> {
    let d = default_settings();
    let mut rows: Vec<El<'a>> = vec![
        text("System — change requires reboot").size(18).into(),
        cycle("Renderer", e, &e.renderer, &d.renderer, &["vulkan", "gles"], |x, v| x.renderer = v),
        boolean("Renderer GLES fallback", e, e.renderer_fallback, d.renderer_fallback, |x, v| x.renderer_fallback = v),
        cycle("Scanout depth", e, &e.depth.to_string(), &d.depth.to_string(), &["8", "10"], |x, v| x.depth = v.parse().unwrap_or(8)),
        boolean("Variable refresh (VRR)", e, e.vrr, d.vrr, |x, v| x.vrr = v),
        cycle("Capture encoder", e, &e.capture_encoder, &d.capture_encoder, &["nvenc", "vaapi"], |x, v| x.capture_encoder = v),
        cycle("Capture codec", e, &e.capture_codec, &d.capture_codec, &["av1", "h265", "h264"], |x, v| x.capture_codec = v),
        cycle("Capture quality", e, &e.capture_quality, &d.capture_quality, &["optimized", "lossless"], |x, v| x.capture_quality = v),
        cycle("Capture fps max", e, &e.capture_refresh_rate_max.to_string(), &d.capture_refresh_rate_max.to_string(), &["30", "60", "90", "120"], |x, v| x.capture_refresh_rate_max = v.parse().unwrap_or(120)),
        cycle("Capture re-encode", e, &e.capture_background_encoder, &d.capture_background_encoder, &["ffmpeg", ""], |x, v| x.capture_background_encoder = v),
        boolean("NVENC readback fallback", e, e.capture_nvenc_allow_readback_fallback, d.capture_nvenc_allow_readback_fallback, |x, v| x.capture_nvenc_allow_readback_fallback = v),
        boolean("Capture variable frame rate", e, e.capture_variable_frame_rate, d.capture_variable_frame_rate, |x, v| x.capture_variable_frame_rate = v),
        textfield("Render node", e, &e.render_node, &d.render_node, |x, v| x.render_node = v),
        textfield("Desktop name", e, &e.desktop_name, &d.desktop_name, |x, v| x.desktop_name = v),
        textfield("Log level", e, &e.log_level, &d.log_level, |x, v| x.log_level = v),
    ];
    // Render-device picker (render nodes only, with estimated GPU names). The
    // free-text "Render node" field above still works for custom paths.
    rows.push(text("Render device").size(14).into());
    let current = devices.iter().find(|r| r.node == e.render_node).map(|r| r.name.as_str());
    rows.push(text(format!("Current: {}", current.unwrap_or("(custom / not listed)"))).size(12).into());
    for r in devices {
        let mut e2 = e.clone();
        e2.render_node = r.node.clone();
        let mark = if r.node == e.render_node { "●" } else { "○" };
        rows.push(
            button(text(format!("{mark} {}  —  {}", r.name, r.node)))
                .width(Length::Fill)
                .on_press(SettingsMessage::Env(e2))
                .style(style::action)
                .into(),
        );
    }
    scrollable(Column::with_children(rows).spacing(8).padding(4)).into()
}
