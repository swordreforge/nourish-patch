//! The persistent status bar — a thin strip at the top of the screen with clock
//! and battery indicators. Auto-hides when idle; shows when the pointer reaches
//! the top edge. Styled to match the overview menu bar (same colour palette).
use iced_core::{Background, Color, Element, Length, Padding, Theme};
use iced_widget::{container, text, Row, Space};
use compositor_support_iced_core_engine_base::{IcedUi, Renderer};

const MUTED: Color = Color { r: 0.46, g: 0.54, b: 0.60, a: 1.0 };
const BG: Color = Color { r: 0.027, g: 0.043, b: 0.059, a: 0.92 };

/// Height of the status bar in physical pixels.
pub const BAR_HEIGHT: i32 = 32;

/// Margin at the top of the screen where pointer motion triggers auto-show (px).
pub const SHOW_MARGIN: i32 = 4;

#[derive(Debug, Clone)]
pub enum StatusBarMessage {
    Clock(String),
    Battery(Option<String>),
}

#[derive(Debug)]
pub struct StatusBar {
    clock: String,
    battery: Option<String>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self { clock: String::new(), battery: None }
    }
}

impl IcedUi for StatusBar {
    type Message = StatusBarMessage;

    fn view(&self) -> Element<'_, Self::Message, Theme, Renderer> {
        let mut items: Vec<Element<'_, Self::Message, Theme, Renderer>> = Vec::new();
        // Left: clock
        let clock = if self.clock.is_empty() { "···".to_string() } else { self.clock.clone() };
        items.push(text(clock).size(12).color(MUTED).into());
        items.push(Space::new().width(Length::Fill).into());
        // Right: battery
        if let Some(bat) = &self.battery {
            items.push(text(bat.clone()).size(12).color(MUTED).into());
        }
        let bar = Row::with_children(items).spacing(12).padding(Padding::from([0, 12]));
        container(bar)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_y(Length::Fill)
            .style(|_t: &Theme| container::Style {
                background: Some(Background::Color(BG)),
                ..Default::default()
            })
            .into()
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            StatusBarMessage::Clock(c) => self.clock = c,
            StatusBarMessage::Battery(b) => self.battery = b,
        }
    }
}
