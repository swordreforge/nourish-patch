//! The Graphics module: anti-aliasing method + every tunable knob, each with a
//! per-zoom weight (`base` + `per_zoom`), plus the activation-zoom threshold.
//!
//! Only the knobs that apply to the SELECTED method are shown. A live status
//! line reports the current zoom and whether AA is running right now. Every knob
//! has a numeric entry (clamped to its range) and a reset-to-default (↺); a
//! "Restore all defaults" button resets the whole config. Edits emit the full
//! `GraphicsAaConfig`; the handler persists it to `preferences.json` and pushes
//! it live to the renderer.
use compositor_developer_environment_graphics_base::base::{AaMethod, GraphicsAaConfig, ZoomKnob};
use compositor_support_iced_core_engine_base::Renderer;
use compositor_configurator_settings_surface_message::message::SettingsMessage;
use compositor_configurator_settings_surface_style::style;
use compositor_configurator_settings_surface_control::control;
use iced_core::{Alignment, Element, Length, Theme};
use iced_widget::{button, column, container, pick_list, row, scrollable, slider, text, text_input, Column};
use std::ops::RangeInclusive;

type El<'a> = Element<'a, SettingsMessage, Theme, Renderer>;

const DEF: GraphicsAaConfig = GraphicsAaConfig::DEFAULT;

fn msg(cfg: &GraphicsAaConfig, set: impl FnOnce(&mut GraphicsAaConfig)) -> SettingsMessage {
    let mut x = *cfg;
    set(&mut x);
    SettingsMessage::SetGraphics(x)
}

/// One numeric control: label, a clamped text entry, a reset-to-default (↺),
/// and a slider — all driving the same field.
fn num<'a>(
    label: &'a str,
    cfg: &GraphicsAaConfig,
    value: f32,
    default: f32,
    range: RangeInclusive<f32>,
    step: f32,
    set: fn(&mut GraphicsAaConfig, f32),
) -> El<'a> {
    let (lo, hi) = (*range.start(), *range.end());
    let c_in = *cfg;
    let entry = text_input("", &format!("{value:.2}"))
        .width(Length::Fixed(66.0))
        .on_input(move |s| {
            // Parse + clamp to the allowed range; keep the current value on a
            // partial/invalid entry.
            let v = s.parse::<f32>().ok().map(|v| v.clamp(lo, hi)).unwrap_or(value);
            msg(&c_in, |x| set(x, v))
        });
    let reset = button(text("↺").size(12))
        .on_press(msg(cfg, |x| set(x, default)))
        .style(control::action);
    let c_sl = *cfg;
    let s = slider(range, value, move |v| msg(&c_sl, |x| set(x, v)))
        .step(step)
        .style(control::slider);
    container(
        column![
            row![
                text(label).size(12).color(style::MUTED).width(Length::Fill),
                entry,
                reset,
            ]
            .align_y(Alignment::Center)
            .spacing(8),
            s,
        ]
        .spacing(6)
        .padding(12),
    )
    .style(style::card)
    .width(Length::Fill)
    .into()
}

/// A knob = its `base` (value at 100% zoom) + its `per_zoom` slope (added per
/// unit of zoom-out).
#[allow(clippy::too_many_arguments)]
fn knob<'a>(
    title: &'a str,
    cfg: &GraphicsAaConfig,
    k: ZoomKnob,
    dflt: ZoomKnob,
    base_range: RangeInclusive<f32>,
    base_step: f32,
    slope_range: RangeInclusive<f32>,
    slope_step: f32,
    set_base: fn(&mut GraphicsAaConfig, f32),
    set_slope: fn(&mut GraphicsAaConfig, f32),
) -> El<'a> {
    column![
        text(title).size(13).color(style::ACCENT),
        num("value at 100% zoom", cfg, k.base, dflt.base, base_range, base_step, set_base),
        num("+ per zoom-out (ramps as you zoom out)", cfg, k.per_zoom, dflt.per_zoom, slope_range, slope_step, set_slope),
    ]
    .spacing(6)
    .into()
}

fn heading<'a>(title: &'a str, sub: &'a str) -> El<'a> {
    column![
        text(title).size(13).color(style::ACCENT),
        text(sub).size(11).color(style::MUTED),
    ]
    .spacing(2)
    .into()
}

fn method_row<'a>(cfg: &GraphicsAaConfig) -> El<'a> {
    let cur = cfg.method.label().to_string();
    let options: Vec<String> = AaMethod::ALL.iter().map(|m| m.label().to_string()).collect();
    let c = *cfg;
    let picker = pick_list(Some(cur), options, |s: &String| s.clone())
        .on_select(move |s: String| {
            let mut x = c;
            if let Some(m) = AaMethod::ALL.iter().find(|m| m.label() == s) {
                x.method = *m;
            }
            SettingsMessage::SetGraphics(x)
        })
        .width(Length::Fixed(200.0))
        .style(control::picklist)
        .menu_style(control::menu);
    let reset_all = button(text("Restore all defaults").size(12))
        .on_press(SettingsMessage::SetGraphics(DEF))
        .style(control::action);
    container(
        row![
            text("Method").width(Length::Fill),
            picker,
            reset_all,
        ]
        .align_y(Alignment::Center)
        .spacing(10)
        .padding(12),
    )
    .style(style::card)
    .width(Length::Fill)
    .into()
}

/// Live status: current zoom + whether AA runs right now (and, if so, the
/// effective per-zoom values). Answers "when is it on, and with what".
fn status<'a>(cfg: &GraphicsAaConfig) -> El<'a> {
    let zoom = compositor_developer_stats_registry_base::base::world_zoom() as f32;
    let eff = cfg.effective(zoom);
    let line1 = format!("Current zoom {zoom:.2}×");
    let (state, col) = if cfg.method == AaMethod::Off {
        ("AA OFF — method is Off".to_string(), style::MUTED)
    } else if eff.active {
        (
            format!(
                "AA ACTIVE · taps {} · spread {:.2} · sharpen {:.2} · lod {:+.2}",
                eff.taps, eff.spread, eff.sharpen, eff.lod_bias
            ),
            style::ACCENT,
        )
    } else {
        (
            format!(
                "AA OFF right now — zoom {zoom:.2}× ≥ threshold {:.2}× (zoom out to enable)",
                cfg.activate_below_zoom
            ),
            style::MUTED,
        )
    };
    container(
        column![text(line1).size(12).color(style::MUTED), text(state).size(12).color(col)].spacing(4).padding(12),
    )
    .style(style::card)
    .width(Length::Fill)
    .into()
}

pub fn build<'a>(cfg: &GraphicsAaConfig) -> El<'a> {
    let mut rows: Vec<El<'a>> = vec![
        column![
            text("GRAPHICS").size(16).color(style::ACCENT),
            text("Anti-aliasing for the pannable world — applied live, weighted by zoom.")
                .size(11)
                .color(style::MUTED),
        ]
        .spacing(4)
        .into(),
        status(cfg),
        method_row(cfg),
        heading(
            "WHEN AA RUNS",
            "AA is OFF above this zoom; it turns on (and ramps) as you zoom out below it.",
        ),
        num(
            "Activate below zoom (× )",
            cfg,
            cfg.activate_below_zoom,
            DEF.activate_below_zoom,
            0.1..=1.5,
            0.05,
            |x, v| x.activate_below_zoom = v,
        ),
        num(
            "Max zoom-out (weight clamp)",
            cfg,
            cfg.max_zoom_out,
            DEF.max_zoom_out,
            1.0..=48.0,
            1.0,
            |x, v| x.max_zoom_out = v,
        ),
    ];

    // Only the knobs that apply to the chosen method.
    match cfg.method {
        AaMethod::Off => rows.push(
            container(text("Select a method above to enable and tune anti-aliasing.").size(12).color(style::MUTED).width(Length::Fill))
                .style(style::card)
                .padding(12)
                .width(Length::Fill)
                .into(),
        ),
        AaMethod::Ssaa => {
            rows.push(heading("SUPERSAMPLE (SSAA)", "N×N box downsample + sharpen. No mip chain."));
            rows.push(knob("Taps / axis", cfg, cfg.taps, DEF.taps, 1.0..=16.0, 1.0, 0.0..=8.0, 0.5, |x, v| x.taps.base = v, |x, v| x.taps.per_zoom = v));
            rows.push(knob("Spread", cfg, cfg.spread, DEF.spread, 0.25..=4.0, 0.05, 0.0..=4.0, 0.1, |x, v| x.spread.base = v, |x, v| x.spread.per_zoom = v));
            rows.push(knob("Sharpen (unsharp)", cfg, cfg.sharpen, DEF.sharpen, 0.0..=3.0, 0.05, 0.0..=2.0, 0.05, |x, v| x.sharpen.base = v, |x, v| x.sharpen.per_zoom = v));
        }
        AaMethod::Trilinear => {
            rows.push(heading("TRILINEAR MIPS", "Mip-averaged minification + optional sharpen."));
            rows.push(knob("LOD bias", cfg, cfg.lod_bias, DEF.lod_bias, -4.0..=4.0, 0.1, -2.0..=2.0, 0.1, |x, v| x.lod_bias.base = v, |x, v| x.lod_bias.per_zoom = v));
            rows.push(knob("Sharpen (unsharp)", cfg, cfg.sharpen, DEF.sharpen, 0.0..=3.0, 0.05, 0.0..=2.0, 0.05, |x, v| x.sharpen.base = v, |x, v| x.sharpen.per_zoom = v));
        }
        AaMethod::Anisotropic => {
            rows.push(heading("ANISOTROPIC", "Mip-based; helps oblique footprints. Level ≈ device max."));
            rows.push(num("Anisotropy (max)", cfg, cfg.aniso, DEF.aniso, 1.0..=16.0, 1.0, |x, v| x.aniso = v));
        }
    }

    scrollable(Column::with_children(rows).spacing(12))
        .height(Length::Fill)
        .into()
}
