//! Section rendering: identity, launch, handler-scoped attribute groups.

use iced_core::{Alignment, Element, Length, Theme};
use iced_widget::{column, container, text};
use compositor_introspection_extraction_window_base::AppHandler;
use compositor_introspection_inference_hint_base::{identity_descriptors, launch_descriptors};
use compositor_support_iced_core_engine_base::Renderer;

use super::attribute_section;
use crate::message::PlaceholderMessage;
use crate::style;
use crate::ui::PlaceholderUi;

pub fn render_identity_section(
    ui: &PlaceholderUi,
) -> Element<'_, PlaceholderMessage, Theme, Renderer> {
    section(ui, "Identity", identity_descriptors())
}

pub fn render_launch_section(
    ui: &PlaceholderUi,
) -> Element<'_, PlaceholderMessage, Theme, Renderer> {
    section(ui, "Launch", launch_descriptors())
}

pub fn render_handler_section(
    ui: &PlaceholderUi,
) -> Element<'_, PlaceholderMessage, Theme, Renderer> {
    let Some(handler_id) = ui.working.active_handler else {
        return empty_section_message(
            "No active handler. Handler-specific settings will appear here \
             when a handler is selected above.",
        );
    };
    let Some(handler) = ui.registry.get(handler_id) else {
        return empty_section_message("Active handler is not registered.");
    };
    let descriptors = handler.attribute_descriptors();
    let title = format!("{} settings", handler_id);
    section_owned(ui, title, descriptors)
}

fn section<'a>(
    ui: &'a PlaceholderUi,
    title: &'static str,
    descriptors: Vec<compositor_introspection_extraction_window_base::AttributeDescriptor>,
) -> Element<'a, PlaceholderMessage, Theme, Renderer> {
    section_owned(ui, title.to_string(), descriptors)
}

fn section_owned<'a>(
    ui: &'a PlaceholderUi,
    title: String,
    descriptors: Vec<compositor_introspection_extraction_window_base::AttributeDescriptor>,
) -> Element<'a, PlaceholderMessage, Theme, Renderer> {
    let title_widget = text(title)
        .size(style::TEXT_SIZE_SECTION)
        .style(|_| iced_widget::text::Style { color: Some(style::TEXT) });

    let mut col = column![title_widget].spacing(10).align_x(Alignment::Start);

    if descriptors.is_empty() {
        col = col.push(
            text("(no attributes)")
                .size(style::TEXT_SIZE_HINT)
                .style(|_| iced_widget::text::Style {
                    color: Some(style::TEXT_HINT),
                }),
        );
    } else {
        for d in descriptors {
            col = col.push(attribute_section::render(ui, &d));
        }
    }

    container(col)
        .padding(style::PAD_MEDIUM)
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced_core::Background::Color(style::PANEL_BG)),
            border: iced_core::Border {
                color: style::BORDER,
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: iced_core::Shadow::default(),
            text_color: Some(style::TEXT),
            snap: true,
        })
        .into()
}

fn empty_section_message<'a>(
    msg: &'static str,
) -> Element<'a, PlaceholderMessage, Theme, Renderer> {
    container(
        text(msg)
            .size(style::TEXT_SIZE_HINT)
            .style(|_| iced_widget::text::Style {
                color: Some(style::TEXT_HINT),
            }),
    )
    .padding(style::PAD_MEDIUM)
    .width(Length::Fill)
    .style(|_| container::Style {
        background: Some(iced_core::Background::Color(style::PANEL_BG)),
        border: iced_core::Border {
            color: style::BORDER,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: iced_core::Shadow::default(),
        text_color: Some(style::TEXT),
        snap: true,
    })
    .into()
}
