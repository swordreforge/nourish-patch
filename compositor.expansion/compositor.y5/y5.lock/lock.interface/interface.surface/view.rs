//! Iced UI for the lock screen.
//!
//! Stores the typed password in a `ZeroString` so it is zeroed on
//! drop. The `process()` translation layer converts `Submit` (from
//! either the Unlock button or the Enter key) into `Attempt(pin)`
//! carrying a cloned `ZeroString` — the clone is also zeroed on drop.

use compositor_support_library_i18n_base_core::t;
use compositor_support_library_pam_worker_base::zero_string;
use iced_core::alignment::{Horizontal, Vertical};
use iced_core::keyboard::key::Named;
use iced_core::keyboard::{self, Key};
use iced_core::{Background, Border, Color, Element, Event as IcedEvent, Length, Padding, Theme};
use iced_widget::{button, column, container, text, Space};
use compositor_support_iced_core_engine_base::ui::EventFlags;
use compositor_support_iced_core_engine_base::{IcedUi, Renderer};

use crate::message::LockMessage;
use zero_string::ZeroString;

const PIN_MAX_LEN: usize = 256;
const CARD_WIDTH: f32 = 360.0;
const DOTS_WIDTH: f32 = 280.0;

pub struct LockSurface {
    pin: ZeroString,
    error: Option<String>,
    submitting: bool,
}

impl LockSurface {
    pub fn new() -> Self {
        Self {
            pin: ZeroString::new(),
            error: None,
            submitting: false,
        }
    }

    /// Called by the compositor when PAM rejects the attempt.
    pub fn fail(&mut self, reason: impl Into<String>) {
        self.error = Some(reason.into());
        self.pin.clear();
        self.submitting = false;
    }

    /// Called by the compositor right after it dispatches `Attempt`
    /// to the PAM worker, so the UI rejects further input until a
    /// response arrives.
    pub fn mark_submitting(&mut self) {
        self.submitting = true;
    }
}

impl Default for LockSurface {
    fn default() -> Self {
        Self::new()
    }
}

impl IcedUi for LockSurface {
    type Message = LockMessage;

    fn update(&mut self, message: Self::Message) {
        match message {
            LockMessage::AppendChar(c) => {
                if !self.submitting && self.pin.len() < PIN_MAX_LEN {
                    self.pin.push(c);
                    self.error = None;
                }
            }
            LockMessage::Backspace => {
                if !self.submitting {
                    self.pin.pop();
                    self.error = None;
                }
            }
            LockMessage::Clear => {
                if !self.submitting {
                    self.pin.clear();
                    self.error = None;
                }
            }
            LockMessage::Submit => {}
            LockMessage::Attempt(_pin) => {
                // `process()` will emit `Attempt` separately if the
                // pin is non-empty. We just mark submitting so any
                // further keypresses are ignored.
                self.submitting = true;
                // if !self.pin.is_empty() {
                // }
                // `process()` produced this for the parent
                // (compositor) to forward to the PAM worker. The
                // carried ZeroString is zeroed when this message is
                // dropped, unless the parent moves it into the worker
                // first.
            }
            LockMessage::AuthFailed(reason) => {
                self.fail(reason);
            }
            LockMessage::AuthSucceeded => {
                // Parent tears down the surface.
            }
        }
    }

    
    fn subscribe(&self) -> EventFlags {
        EventFlags::KEYBOARD
    }

    fn event_process(&self, event: &IcedEvent) -> Vec<Self::Message> {
        let IcedEvent::Keyboard(keyboard::Event::KeyPressed { key, text, .. }) = event else {
            return Vec::new();
        };

        if self.submitting {
            return Vec::new();
        }

        match key {
            Key::Named(Named::Enter) => vec![LockMessage::Submit],
            Key::Named(Named::Backspace) => vec![LockMessage::Backspace],
            Key::Named(Named::Escape) => vec![LockMessage::Clear],
            Key::Named(_) => Vec::new(),
            Key::Character(_) | Key::Unidentified => {
                let Some(t) = text else { return Vec::new() };
                t.chars()
                    .filter(|c| !c.is_control())
                    .map(LockMessage::AppendChar)
                    .collect()
            }
        }
    }
    
    fn process(&self, message: &Self::Message) -> Vec<Self::Message> {
        match message {
            LockMessage::Submit => {
                if self.submitting || self.pin.is_empty() {
                    Vec::new()
                } else {
                    // ZeroString::clone produces another ZeroString
                    // that is zeroed when this message is dropped.
                    vec![LockMessage::Attempt(self.pin.clone())]
                }
            }
            _ => Vec::new(),
        }
    }

    fn view(&self) -> Element<'_, Self::Message, Theme, Renderer> {
        let dot_display: Element<_, _, _> = if self.pin.is_empty() {
            text(t!("Enter password"))
                .size(15)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.4))
                .into()
        } else {
            text("●".repeat(self.pin.char_count()))
                .size(22)
                .color(Color::from_rgb(0.95, 0.95, 0.95))
                .into()
        };

        let input_box = container(dot_display)
            .width(DOTS_WIDTH)
            .height(48)
            .padding(Padding::from([10, 16]))
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .style(|_theme: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
                border: Border {
                    color: if self.error.is_some() {
                        Color::from_rgba(0.95, 0.35, 0.35, 0.8)
                    } else {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.15)
                    },
                    width: 1.5,
                    radius: 8.0.into(),
                },
                text_color: Some(Color::WHITE),
                ..Default::default()
            });

        let status: Element<_, _, _> = match &self.error {
            Some(msg) => text(msg.as_str())
                .size(13)
                .color(Color::from_rgb(0.95, 0.45, 0.45))
                .into(),
            None => text(t!("Press Enter to unlock"))
                .size(12)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.4))
                .into(),
        };

        let unlock_button = button(
            text(t!("Unlock"))
                .size(14)
                .align_x(Horizontal::Center)
                .width(Length::Fill),
        )
        .width(DOTS_WIDTH)
        .padding(Padding::from([10, 0]))
        .on_press_maybe(if self.submitting || self.pin.is_empty() {
            None
        } else {
            Some(LockMessage::Submit)
        });

        let card = container(
            column![
                text(t!("Locked")).size(28).color(Color::WHITE),
                Space::new().height(28),
                input_box,
                Space::new().height(10),
                container(status).height(18),
                Space::new().height(20),
                unlock_button,
            ]
            .align_x(Horizontal::Center),
        )
        .width(CARD_WIDTH)
        .padding(Padding::from([36, 32]))
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.10, 0.10, 0.12, 0.92))),
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
                width: 1.0,
                radius: 16.0.into(),
            },
            ..Default::default()
        });

        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                // Opaque backdrop. Non-negotiable for privacy — the
                // desktop underneath must not be visible.
                background: Some(Background::Color(Color::from_rgb(0.04, 0.04, 0.06))),
                ..Default::default()
            })
            .into()
    }

}
