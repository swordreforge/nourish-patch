//! Audio tab: list output sinks, pick the default (● marker), per-sink volume.
use compositor_y5_audio_controller_interface::interface::AudioState;
use compositor_support_iced_core_engine_base::Renderer;
use compositor_configurator_settings_surface_message::message::SettingsMessage;
use compositor_configurator_settings_surface_style::style;
use iced_core::{Element, Length, Theme};
use iced_widget::{button, row, scrollable, slider, text, Column};

pub fn build<'a>(a: &'a AudioState) -> Element<'a, SettingsMessage, Theme, Renderer> {
    let mut rows: Vec<Element<'a, SettingsMessage, Theme, Renderer>> = vec![text("Audio output").size(18).into()];
    if a.sinks.is_empty() {
        rows.push(text("No audio outputs found.").into());
    }
    for s in &a.sinks {
        let pick = s.name.clone();
        let vol_name = s.name.clone();
        let mark = if s.is_default { "●" } else { "○" };
        let label = if s.description.is_empty() { s.name.clone() } else { s.description.clone() };
        rows.push(
            row![
                button(text(format!("{mark} {label}"))).width(Length::Fill).on_press(SettingsMessage::SetDefaultSink(pick)).style(style::action),
                slider(0.0..=1.0, s.volume as f32, move |v| SettingsMessage::SetSinkVolume(vol_name.clone(), v)).step(0.02f32).width(Length::Fixed(150.0)),
            ]
            .spacing(8)
            .into(),
        );
    }
    scrollable(Column::with_children(rows).spacing(6).padding(4)).into()
}
