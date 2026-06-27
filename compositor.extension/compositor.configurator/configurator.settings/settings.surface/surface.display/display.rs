//! The Display tab: a scroll list of advertised resolution/refresh modes; the
//! Keep/Revert confirm bar lives in the panel chrome (pinned to the bottom).
use compositor_orchestration_driver_output_base::base::ModeInfo;
use compositor_support_iced_core_engine_base::Renderer;
use compositor_configurator_settings_surface_message::message::SettingsMessage;
use compositor_configurator_settings_surface_style::style;
use iced_core::{Element, Length, Theme};
use iced_widget::{button, column, scrollable, text, Column};

pub fn build<'a>(modes: &[ModeInfo], current: Option<ModeInfo>) -> Element<'a, SettingsMessage, Theme, Renderer> {
    if modes.is_empty() {
        return column![
            text("Display").size(18),
            text("No advertised modes (running nested / no DRM backend)."),
        ]
        .spacing(8)
        .into();
    }
    let mut rows: Vec<Element<'a, SettingsMessage, Theme, Renderer>> = vec![text("Display").size(18).into()];
    for m in modes {
        let mark = if Some(*m) == current { "  • current" } else { "" };
        let label = format!("{}×{} @ {:.2} Hz{mark}", m.width, m.height, m.refresh_mhz as f32 / 1000.0);
        rows.push(button(text(label)).width(Length::Fill).on_press(SettingsMessage::PickMode(*m)).style(style::action).into());
    }
    scrollable(Column::with_children(rows).spacing(6).padding(4)).into()
}
