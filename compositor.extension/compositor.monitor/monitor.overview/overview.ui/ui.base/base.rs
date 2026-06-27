//! The overview top menu bar: a thin, full-width iced bar — `World` and `Layout`
//! on the left, `Settings` on the right. Presentational; selecting a tab emits
//! [`OverviewMessage::Select`], which the compositor applies to its slot.

use iced_core::{Alignment, Background, Border, Color, Element, Length, Padding, Theme};
use iced_widget::{button, container, row, text, Space};
use compositor_support_iced_core_engine_base::{IcedUi, Renderer};

/// Which menu section a click selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    World,
    Layout,
    Settings,
}

/// Messages the menu bar emits. Only `Select` is meaningful to the compositor.
#[derive(Debug, Clone)]
pub enum OverviewMessage {
    Select(Section),
}

/// The menu-bar UI instance.
pub struct OverviewMenu {
    selected: Section,
}

impl OverviewMenu {
    /// Opens on `Layout` (the window grid) by default.
    pub fn new() -> Self {
        Self { selected: Section::Layout }
    }

    fn tab<'a>(
        &self,
        label: &'a str,
        section: Section,
    ) -> Element<'a, OverviewMessage, Theme, Renderer> {
        let active = self.selected == section;
        button(text(label).size(16.0))
            .padding(Padding::from([6, 14]))
            .style(move |_theme: &Theme, _status| button::Style {
                background: active
                    .then(|| Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.16))),
                text_color: if active {
                    Color::WHITE
                } else {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.7)
                },
                border: Border { radius: 6.0.into(), ..Border::default() },
                ..button::Style::default()
            })
            .on_press(OverviewMessage::Select(section))
            .into()
    }
}

impl IcedUi for OverviewMenu {
    type Message = OverviewMessage;

    fn view(&self) -> Element<'_, Self::Message, Theme, Renderer> {
        let bar = row![
            self.tab("World", Section::World),
            self.tab("Layout", Section::Layout),
            Space::new().width(Length::Fill),
            self.tab("Settings", Section::Settings),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .padding(Padding::from([0, 16]));

        container(bar)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgba(0.05, 0.05, 0.07, 0.85))),
                text_color: Some(Color::WHITE),
                ..container::Style::default()
            })
            .into()
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            OverviewMessage::Select(section) => self.selected = section,
        }
    }
}
