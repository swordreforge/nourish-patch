//! The System module: editable `Environment` fields (settings.json). Enum fields
//! are dropdowns, booleans are pill toggles, free strings use a field. Each edit
//! emits the full edited struct; the handler writes settings.json + reboot banner.
use compositor_developer_environment_config_base::base::Environment;
use compositor_configurator_hardware_gpu_base::base::RenderDevice;
use compositor_support_iced_core_engine_base::Renderer;
use compositor_configurator_settings_surface_message::message::SettingsMessage;
use compositor_configurator_settings_surface_style::style;
use compositor_configurator_settings_surface_control::control;
use iced_core::{Alignment, Element, Length, Theme};
use iced_widget::{column, container, pick_list, row, scrollable, text, text_input, toggler, Column};

type El<'a> = Element<'a, SettingsMessage, Theme, Renderer>;

fn cell<'a>(label: &'a str, control_el: El<'a>) -> El<'a> {
    container(row![text(label).width(Length::Fill), control_el].align_y(Alignment::Center).spacing(10).padding(12))
        .style(style::card).width(Length::Fill).into()
}

fn opts(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|s| s.to_string()).collect()
}

fn choice<'a>(label: &'a str, e: &Environment, cur: String, options: Vec<String>, set: fn(&mut Environment, String)) -> El<'a> {
    let e2 = e.clone();
    let picker = pick_list(Some(cur), options, |s: &String| s.clone())
        .on_select(move |s: String| { let mut x = e2.clone(); set(&mut x, s); SettingsMessage::Env(x) })
        .width(Length::Fixed(170.0))
        .style(control::picklist)
        .menu_style(control::menu);
    cell(label, picker.into())
}

fn boolean<'a>(label: &'a str, e: &Environment, v: bool, set: fn(&mut Environment, bool)) -> El<'a> {
    let e2 = e.clone();
    let t = toggler(v).on_toggle(move |b| { let mut x = e2.clone(); set(&mut x, b); SettingsMessage::Env(x) }).style(control::toggler);
    cell(label, t.into())
}

fn textfield<'a>(label: &'a str, e: &'a Environment, val: &'a str, set: fn(&mut Environment, String)) -> El<'a> {
    let e2 = e.clone();
    let f = text_input(label, val).width(Length::Fixed(220.0)).on_input(move |s| { let mut x = e2.clone(); set(&mut x, s); SettingsMessage::Env(x) });
    cell(label, f.into())
}

pub fn build<'a>(e: &'a Environment, devices: &'a [RenderDevice]) -> El<'a> {
    let head = column![
        text("SYSTEM").size(16).color(style::ACCENT),
        text("Build, runtime, and capture — changes require a reboot.").size(11).color(style::MUTED),
    ].spacing(4);
    let mut rows: Vec<El<'a>> = vec![head.into()];
    rows.push(choice("Renderer", e, e.renderer.clone(), opts(&["vulkan", "gles"]), |x, v| x.renderer = v));
    rows.push(boolean("Renderer GLES fallback", e, e.renderer_fallback, |x, v| x.renderer_fallback = v));
    rows.push(choice("Scanout depth", e, e.depth.to_string(), opts(&["8", "10"]), |x, v| x.depth = v.parse().unwrap_or(8)));
    rows.push(boolean("Variable refresh (VRR)", e, e.vrr, |x, v| x.vrr = v));
    rows.push(choice("Capture encoder", e, e.capture_encoder.clone(), opts(&["nvenc", "vaapi"]), |x, v| x.capture_encoder = v));
    rows.push(choice("Capture codec", e, e.capture_codec.clone(), opts(&["av1", "h265", "h264"]), |x, v| x.capture_codec = v));
    rows.push(choice("Capture quality", e, e.capture_quality.clone(), opts(&["optimized", "lossless"]), |x, v| x.capture_quality = v));
    rows.push(choice("Capture fps max", e, e.capture_refresh_rate_max.to_string(), opts(&["30", "60", "90", "120"]), |x, v| x.capture_refresh_rate_max = v.parse().unwrap_or(120)));
    rows.push(choice("Capture re-encode", e, e.capture_background_encoder.clone(), opts(&["ffmpeg", ""]), |x, v| x.capture_background_encoder = v));
    rows.push(boolean("NVENC readback fallback", e, e.capture_nvenc_allow_readback_fallback, |x, v| x.capture_nvenc_allow_readback_fallback = v));
    rows.push(boolean("Capture variable frame rate", e, e.capture_variable_frame_rate, |x, v| x.capture_variable_frame_rate = v));
    rows.push(textfield("Desktop name", e, &e.desktop_name, |x, v| x.desktop_name = v));
    rows.push(textfield("Log level", e, &e.log_level, |x, v| x.log_level = v));
    // Render device: dropdown of detected render nodes (estimated GPU names).
    if !devices.is_empty() {
        let cur = devices.iter().find(|r| r.node == e.render_node).map(|r| r.name.clone()).unwrap_or_else(|| e.render_node.clone());
        let names: Vec<String> = devices.iter().map(|r| r.name.clone()).collect();
        let devs = devices.to_vec();
        let e3 = e.clone();
        let picker = pick_list(Some(cur), names, |s: &String| s.clone())
            .on_select(move |name: String| {
                let mut x = e3.clone();
                if let Some(d) = devs.iter().find(|r| r.name == name) { x.render_node = d.node.clone(); }
                SettingsMessage::Env(x)
            })
            .width(Length::Fixed(220.0)).style(control::picklist).menu_style(control::menu);
        rows.push(cell("Render device", picker.into()));
    }
    scrollable(Column::with_children(rows).spacing(10)).height(Length::Fill).into()
}
