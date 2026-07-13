//! Settings mode: handler picker + attribute sections + Save/Cancel.

pub mod attribute_section;
pub mod attribute_widget;
pub mod handler_picker;
pub mod sections;

use compositor_support_library_i18n_base_core::t;
use iced_core::{Alignment, Element, Length, Theme};
use iced_widget::{button, column, container, row, text};
use compositor_support_iced_core_engine_base::Renderer;

use crate::message::PlaceholderMessage;
use crate::style;
use crate::ui::PlaceholderUi;

pub fn render(ui: &PlaceholderUi) -> Element<'_, PlaceholderMessage, Theme, Renderer> {
    let header = row![
        text(t!("Settings"))
            .size(style::TEXT_SIZE_TITLE)
            .style(|_| iced_widget::text::Style { color: Some(style::TEXT) }),
        button(text(t!("Cancel")).size(style::TEXT_SIZE_BODY))
            .padding(style::PAD_SMALL)
            .on_press(PlaceholderMessage::CancelSettings),
        button(text(t!("Save")).size(style::TEXT_SIZE_BODY))
            .padding(style::PAD_SMALL)
            .on_press(PlaceholderMessage::SaveClicked {
                updated_plan: Box::new(ui.working.clone()),
            }),
        button(text(t!("Restore")).size(style::TEXT_SIZE_BODY))
            .padding(style::PAD_SMALL)
            .on_press(PlaceholderMessage::RestoreClicked{}),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let handler_picker = handler_picker::render(ui);

    let identity = sections::render_identity_section(ui);
    let launch = sections::render_launch_section(ui);
    let handler_scoped = sections::render_handler_section(ui);

    let body = column![
        header,
        handler_picker,
        identity,
        launch,
        handler_scoped,
    ]
    .spacing(16)
    .align_x(Alignment::Start);

    container(body)
        .padding(style::PAD_LARGE)
        .width(Length::Fill)
        .into()
}
