//! [`GroupUi`] — the iced UI instance for a group surface.

use iced_core::keyboard::{self, Key, key::Named};
use iced_core::{Element, Theme};
use compositor_support_iced_core_engine_base::ui::EventFlags;
use compositor_support_iced_core_engine_base::{IcedEvent, IcedUi, Renderer};

use crate::message::GroupMessage;
use crate::mode::Mode;
use crate::view;

#[derive(Clone, Debug)]
pub struct GroupUi {
    /// Whether the group is expanded (full bounding box) or collapsed (chip).
    pub mode: Mode,
    /// The committed group name shown in the header.
    pub name: String,
    /// Whether the name is currently being edited.
    pub editing: bool,
    /// The in-progress name while editing. Committed to `name` on Submit.
    pub draft: String,
}

impl GroupUi {
    pub fn new(mode: Mode, name: impl Into<String>) -> Self {
        Self {
            mode,
            name: name.into(),
            editing: false,
            draft: String::new(),
        }
    }

    /// True when the group is folded into the compact chip.
    pub fn is_collapsed(&self) -> bool {
        matches!(self.mode, Mode::Collapse)
    }

    /// The text the header should display right now (draft while editing).
    pub fn shown_name(&self) -> &str {
        if self.editing {
            &self.draft
        } else {
            &self.name
        }
    }
}

impl IcedUi for GroupUi {
    type Message = GroupMessage;

    fn update(&mut self, message: Self::Message) {
        match message {
            GroupMessage::Collapse => self.mode = Mode::Collapse,
            GroupMessage::Show => self.mode = Mode::Show,

            GroupMessage::StartEdit => {
                if !self.editing {
                    self.editing = true;
                    // Seed the draft with the current name so editing feels
                    // like editing, not retyping.
                    self.draft = self.name.clone();
                }
            }
            GroupMessage::AppendChar(c) => {
                if self.editing {
                    self.draft.push(c);
                }
            }
            GroupMessage::Backspace => {
                if self.editing {
                    self.draft.pop();
                }
            }
            GroupMessage::Submit => {
                if self.editing {
                    let trimmed = self.draft.trim();
                    if !trimmed.is_empty() {
                        self.name = trimmed.to_string();
                    }
                    self.editing = false;
                    // NOTE: `draft` is intentionally left intact so `process`
                    // can read the committed value regardless of whether it
                    // runs before or after `update`. It is reseeded on the
                    // next `StartEdit`.
                }
            }
            GroupMessage::Clear => {
                // Escape: abandon the edit, keep the previous name.
                self.editing = false;
                self.draft.clear();
            }

            // Outward-only: the UI emits this to notify the compositor of a
            // committed rename. Nothing to change locally.
            GroupMessage::Renamed(_) => {}
            // Inbound: compositor sets the canonical name (e.g. external rename).
            GroupMessage::SetName(name) => {
                self.name = name;
                if !self.editing {
                    self.draft.clear();
                }
            }
        }
    }

    /// Only listen to the keyboard while the name is being edited, so we don't
    /// swallow keystrokes meant for the windows inside the group.
    fn subscribe(&self) -> EventFlags {
        if self.editing {
            EventFlags::KEYBOARD
        } else {
            EventFlags::empty()
        }
    }

    /// Map keyboard events to editing messages. Ignored entirely unless we are
    /// currently editing.
    fn event_process(&self, event: &IcedEvent) -> Vec<Self::Message> {
        if !self.editing {
            return Vec::new();
        }

        let IcedEvent::Keyboard(keyboard::Event::KeyPressed { key, text, .. }) = event else {
            return Vec::new();
        };

        match key {
            Key::Named(Named::Enter) => vec![GroupMessage::Submit],
            Key::Named(Named::Backspace) => vec![GroupMessage::Backspace],
            Key::Named(Named::Escape) => vec![GroupMessage::Clear],
            // Space arrives as a named key but is a legitimate name character.
            Key::Named(Named::Space) => vec![GroupMessage::AppendChar(' ')],
            Key::Named(_) => Vec::new(),
            Key::Character(_) | Key::Unidentified => {
                let Some(t) = text else { return Vec::new() };
                t.chars()
                    .filter(|c| !c.is_control())
                    .map(GroupMessage::AppendChar)
                    .collect()
            }
        }
    }

    /// Derive follow-up messages. A committed `Submit` produces an outward
    /// [`GroupMessage::Renamed`] carrying the new name for the compositor to
    /// persist. Reads `draft` (which `update` deliberately leaves intact on
    /// submit) so it is correct whether `process` runs before or after
    /// `update`.
    fn process(&self, message: &Self::Message) -> Vec<Self::Message> {
        match message {
            GroupMessage::Submit => {
                let trimmed = self.draft.trim();
                if trimmed.is_empty() {
                    Vec::new()
                } else {
                    vec![GroupMessage::Renamed(trimmed.to_string())]
                }
            }
            _ => Vec::new(),
        }
    }

    fn view(&self) -> Element<'_, Self::Message, Theme, Renderer> {
        view::root_view(self)
    }
}
