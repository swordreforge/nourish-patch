//! ...
//! New: `TextInputChanged(String)` lets iced's text_input write back to UI state.

use iced_core::{
    Background, Border, Color, Element, Length, Padding, Shadow, Theme, Vector,
    alignment,
};
use iced_widget::{button, column, container, row, scrollable, text, text_input};
use compositor_support_iced_core_engine_base::{IcedUi, Renderer};

#[derive(Debug, Clone)]
pub enum OutgoingMessage {
    IncrementClicked,
    ResetClicked,
    ItemClicked(usize),
    TextInputChanged(String),
    TextInputSubmitted,

    SmithayTick(u64),
    SmithayReset,
    SmithayLabel(String),
}

#[derive(Debug, Clone)]
pub struct CounterUi {
    pub count: u32,
    pub last_tick: Option<u64>,
    pub status: String,
    pub last_clicked_item: Option<usize>,
    pub input_text: String,
    pub submitted_history: Vec<String>,
}

impl Default for CounterUi {
    fn default() -> Self {
        Self {
            count: 0,
            last_tick: None,
            status: "ready".to_string(),
            last_clicked_item: None,
            input_text: String::new(),
            submitted_history: Vec::new(),
        }
    }
}

impl IcedUi for CounterUi {
    type Message = OutgoingMessage;

    fn update(&mut self, message: Self::Message) {
        match message {
            OutgoingMessage::IncrementClicked => {
                self.count += 1;
                self.status = format!("incremented to {}", self.count);
            }
            OutgoingMessage::ResetClicked => {
                self.count = 0;
                self.status = "reset (by user)".to_string();
            }
            OutgoingMessage::ItemClicked(i) => {
                self.last_clicked_item = Some(i);
                self.status = format!("clicked item #{i}");
            }
            OutgoingMessage::TextInputChanged(s) => {
                self.input_text = s;
                self.status = format!("typing: {} chars", self.input_text.len());
            }
            OutgoingMessage::TextInputSubmitted => {
                if !self.input_text.is_empty() {
                    self.submitted_history.push(self.input_text.clone());
                    self.status = format!("submitted: {}", self.input_text);
                    self.input_text.clear();
                }
            }
            OutgoingMessage::SmithayTick(n) => {
                self.last_tick = Some(n);
            }
            OutgoingMessage::SmithayReset => {
                self.count = 0;
                self.status = "reset (by compositor)".to_string();
            }
            OutgoingMessage::SmithayLabel(s) => {
                self.status = s;
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message, Theme, Renderer> {
        let title = text("DMABUF test panel").size(20);

        let counter_row = row![
            text(format!("Count: {}", self.count)).size(28),
            button(text("Increment").size(16))
                .padding(Padding { top: 6.0, right: 14.0, bottom: 6.0, left: 14.0 })
                .on_press(OutgoingMessage::IncrementClicked),
            button(text("Reset").size(16))
                .padding(Padding { top: 6.0, right: 14.0, bottom: 6.0, left: 14.0 })
                .on_press(OutgoingMessage::ResetClicked),
        ]
            .spacing(12)
            .align_y(alignment::Vertical::Center);

        let tick_line = match self.last_tick {
            Some(n) => text(format!("Last smithay tick: {n}")).size(13),
            None => text("Last smithay tick: (none yet)").size(13),
        };

        let status_line = text(format!("Status: {}", self.status)).size(13);

        let clicked_line = match self.last_clicked_item {
            Some(i) => text(format!("Last item clicked: #{i}")).size(13),
            None => text("Last item clicked: (none yet)").size(13),
        };

        // Text input field for keyboard testing.
        let input_field = text_input("type something and press Enter…", &self.input_text)
            .on_input(OutgoingMessage::TextInputChanged)
            .on_submit(OutgoingMessage::TextInputSubmitted)
            .padding(Padding { top: 8.0, right: 10.0, bottom: 8.0, left: 10.0 })
            .size(14);

        let history_label = text(format!("Submitted ({} total):", self.submitted_history.len()))
            .size(12);

        let mut history_col = column![]
            .spacing(2);
        for (i, entry) in self.submitted_history.iter().enumerate().rev().take(5) {
            history_col = history_col.push(
                text(format!("  {i}: {entry}")).size(12),
            );
        }

        // Scrollable list of items as before.
        let mut items = column![]
            .spacing(4)
            .padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 4.0 });
        for i in 0..30 {
            items = items.push(
                button(text(format!("item #{i}")).size(14))
                    .padding(Padding { top: 4.0, right: 10.0, bottom: 4.0, left: 10.0 })
                    .width(Length::Fill)
                    .on_press(OutgoingMessage::ItemClicked(i)),
            );
        }
        let scroll_area = scrollable(items)
            .height(Length::Fill)
            .width(Length::Fill);

        let body = column![
            title,
            counter_row,
            tick_line,
            status_line,
            clicked_line,
            input_field,
            history_label,
            history_col,
            scroll_area,
        ]
            .spacing(10)
            .align_x(alignment::Horizontal::Left);

        container(body)
            .padding(Padding { top: 16.0, right: 18.0, bottom: 16.0, left: 18.0 })
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(Background::Color(Color::from_rgb(0.10, 0.11, 0.14))),
                border: Border {
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.10),
                    width: 1.0,
                    radius: 12.0.into(),
                },
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.40),
                    offset: Vector::new(0.0, 6.0),
                    blur_radius: 24.0,
                },
                text_color: Some(Color::from_rgb(0.92, 0.94, 0.97)),
                snap: true,
            })
            .align_x(alignment::Horizontal::Left)
            .align_y(alignment::Vertical::Top)
            .into()
    }
}