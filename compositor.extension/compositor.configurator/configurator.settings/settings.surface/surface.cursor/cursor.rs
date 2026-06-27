//! The Cursor tab: live pointer sensitivity + touchpad natural-scroll. Both apply
//! immediately (preferences inline-reload). Each has a Reset to its default.
use compositor_support_iced_core_engine_base::Renderer;
use compositor_configurator_settings_surface_message::message::SettingsMessage;
use compositor_configurator_settings_surface_style::style;
use iced_core::{Element, Length, Theme};
use iced_widget::{button, column, row, slider, text};

pub fn build<'a>(sensitivity: f32, natural: bool) -> Element<'a, SettingsMessage, Theme, Renderer> {
    let sens = row![
        text(format!("Pointer sensitivity: {sensitivity:.2}×")).width(Length::Fill),
        button(text("Reset")).on_press(SettingsMessage::Cursor(1.0)).style(style::action),
    ]
    .spacing(8);
    let scroll = row![
        button(text(format!(
            "Natural scrolling (touchpad): {}",
            if natural { "on" } else { "off" }
        )))
        .width(Length::Fill)
        .on_press(SettingsMessage::NaturalScroll(!natural))
        .style(style::action),
        button(text("Reset")).on_press(SettingsMessage::NaturalScroll(true)).style(style::action),
    ]
    .spacing(8);
    column![
        text("Cursor").size(18),
        sens,
        slider(0.2..=3.0, sensitivity, SettingsMessage::Cursor).step(0.05f32),
        scroll,
    ]
    .spacing(14)
    .padding(4)
    .into()
}
