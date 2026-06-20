use crate::app::CompositorSnapshot;
use iced_core::{
    Alignment, Background, Border, Color, Element, Length, Padding, Point, Shadow, Theme, Vector,
    alignment,
};
use iced_runtime::Task;
use iced_wgpu::Renderer;
use iced_widget::{Container, Stack, button, column, container, row, text};
use crate::selection::{ScaleToFitOption, SelectionAction, SelectionState};

pub struct Overlay {
    pub clicks: u32,
    pub last_pressed: Option<u8>,
    pub mode: Mode,
    pub selection_count: u32,
    pub selection: SelectionState,
    pub cursor_position: Option<Point>,
}

pub enum Mode {
    Selecting,
    None,
}

#[derive(Debug, Clone)]
pub enum Message {
    ButtonPressed(u8),
    SelectionAction(SelectionAction),
    SelectNotify(i32),
    SelectionClicked(SelectionAction),
    ClickedOutside,
    ShiftChanged(bool),
    AltChanged(bool),
    ExecuteSelection(Vec<SelectionAction>, bool),
    ExecuteScaleToFit(ScaleToFitOption),
    CursorMoved(Point),
}

impl Default for Overlay {
    fn default() -> Self {
        Self {
            selection_count: 0,
            clicks: 0,
            last_pressed: None,
            mode: Mode::None,
            selection: SelectionState::default(),
            cursor_position: None,
        }
    }
}

impl Overlay {
    pub fn with_snapshot(_snapshot: CompositorSnapshot) -> Self {
        Self::default()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CursorMoved(point) => {
                self.cursor_position = Some(point);
            }
            Message::ButtonPressed(n) => {
                self.clicks += 1;
                self.last_pressed = Some(n);
                info!("button pressed button={n} total_clicks={}", self.clicks);
            }
            Message::SelectNotify(size) => {
                self.mode = if size == 0 { Mode::None } else { Mode::Selecting };
                self.selection_count = size as u32;
            }
            Message::ShiftChanged(held) => self.selection.shift_held = held,
            Message::AltChanged(held) => self.selection.alt_held = held,
            Message::SelectionClicked(action) => {
                if self.selection.shift_held {
                    self.handle_shift_click(action);
                } else {
                    self.handle_plain_click(action);
                }
            }
            Message::ClickedOutside => self.selection.clear(),
            Message::ExecuteSelection(_, _) => {
                self.selection.clear();
            }
            _ => {}
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message, Theme, Renderer> {
        // Build the bar (or nothing) as before
        let bar_element: Element<'_, Message, Theme, Renderer> = match self.mode {
            Mode::None => container(column!()).into(),
            Mode::Selecting => {
                let bar = self.selection();
                container(bar)
                    .padding(Padding { top: 10.0, right: 16.0, bottom: 10.0, left: 16.0 })
                    .style(|_theme| container::Style {
                        snap: true,
                        background: Some(Background::Color(Color::WHITE)),
                        border: Border {
                            color: Color::from_rgba(0.0, 0.0, 0.0, 0.08),
                            width: 1.0,
                            radius: 14.0.into(),
                        },
                        shadow: Shadow {
                            color: Color::from_rgba(0.0, 0.0, 0.0, 0.18),
                            offset: Vector::new(0.0, 4.0),
                            blur_radius: 18.0,
                        },
                        text_color: Some(Color::from_rgb(0.1, 0.12, 0.16)),
                    })
                    .into()
            }
        };

        // The bottom-anchored bar layer
        let bar_layer = container(bar_element)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Bottom)
            .padding(Padding { top: 0.0, right: 0.0, bottom: 8.0, left: 0.0 });

        // The cursor box layer, positioned absolutely via padding trick
        let cursor_layer: Element<'_, Message, Theme, Renderer> =
            if let Some(pos) = self.cursor_position {
                const BOX_SIZE: f32 = 16.0;
                // Center the box on the cursor position
                let left = (pos.x - BOX_SIZE / 2.0).max(0.0);
                let top = (pos.y - BOX_SIZE / 2.0).max(0.0);

                let dot = container(column!())
                    .width(Length::Fixed(BOX_SIZE))
                    .height(Length::Fixed(BOX_SIZE))
                    .style(|_theme| container::Style {
                        snap: true,
                        background: Some(Background::Color(Color::from_rgb(1.0, 0.2, 0.2))),
                        border: Border {
                            color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                            width: 1.0,
                            radius: 3.0.into(),
                        },
                        shadow: Shadow::default(),
                        text_color: None,
                    });

                container(dot)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(Padding { top, left, right: 0.0, bottom: 0.0 })
                    .align_x(alignment::Horizontal::Left)
                    .align_y(alignment::Vertical::Top)
                    .into()
            } else {
                container(column!()).into()
            };

        // Stack: bar at bottom, cursor box on top
        Stack::new()
            .push(bar_layer)
            .push(cursor_layer)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}