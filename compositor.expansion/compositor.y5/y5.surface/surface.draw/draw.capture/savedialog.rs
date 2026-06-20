//! `SaveDialog` — shown when a capture stops: Save / Save As / Discard.
//! Its own small centered screen-space instance.

use iced_core::{Alignment, Background, Border, Element, Length, Shadow, Theme};
use iced_widget::{button, column, container, row, text};
use compositor_y5_graphic_capture_session::message::CaptureMessage;
use compositor_support_iced_core_engine_base::{IcedUi, Renderer};

use crate::style;

pub struct SaveDialog {
    /// Label for the artifact ("Screenshot" / "Video").
    pub kind_label: &'static str,
}

impl SaveDialog {
    pub fn new(kind_label: &'static str) -> Self {
        Self { kind_label }
    }
}

impl IcedUi for SaveDialog {
    type Message = CaptureMessage;

    fn update(&mut self, _message: Self::Message) {}

    fn view(&self) -> Element<'_, Self::Message, Theme, Renderer> {
        let title = text(format!("{} captured", self.kind_label))
            .size(20)
            .color(style::TEXT);

        let hint = text("Save to the default folder, choose a location, or discard.")
            .size(13)
            .color(style::TEXT_DIM);

        let buttons = row![
            button(text("Save").size(15).color(style::TEXT))
                .padding([8, 18])
                .on_press(CaptureMessage::SaveDefault)
                .style(style::button_with(style::ACCENT)),
            button(text("Save As…").size(15).color(style::TEXT))
                .padding([8, 18])
                .on_press(CaptureMessage::SaveAs)
                .style(style::button_with(style::BUTTON_BG)),
            button(text("Discard").size(15).color(style::TEXT))
                .padding([8, 18])
                .on_press(CaptureMessage::Discard)
                .style(style::button_with(style::STOP_BG)),
        ]
        .spacing(12);

        let panel = container(
            column![title, hint, buttons]
                .spacing(14)
                .align_x(Alignment::Center),
        )
        .padding(24)
        .style(|_t| container::Style {
            background: Some(Background::Color(style::PANEL_BG)),
            text_color: Some(style::TEXT),
            border: Border {
                color: style::ACCENT,
                width: 1.0,
                radius: style::RADIUS.into(),
            },
            shadow: Shadow::default(),
            snap: true,
        });

        container(panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
    }
}
