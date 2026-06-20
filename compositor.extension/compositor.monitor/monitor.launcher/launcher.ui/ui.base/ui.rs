//! [`Launcher`] — the iced UI instance.
//!
//! ### Architecture (redux-shaped, with subscribe + event_process)
//!
//! - **`subscribe`** — bitflag of which iced event categories to
//!   observe; checked before phase 0.
//! - **`event_process(event)`** — pure `iced_event → Vec<message>`;
//!   runs in phase 0 before iced's own event dispatch.
//! - **`view`** — pure render.
//! - **`update(message)`** — pure reducer; mutates state, never re-emits.
//! - **`process(message)`** — pure follow-up derivation; unused here
//!   because `event_process` already handles every key directly.
//!
//! ### Carousel scrolling
//!
//! The view shows [`style::CAROUSEL_VISIBLE`] cells at once.
//! `scroll_offset` is the index into `visible` shown in the leftmost
//! slot. The cursor moves within `[scroll_offset, scroll_offset +
//! CAROUSEL_VISIBLE)`; pressing past either edge shifts the scroll
//! window by exactly one — like a typical edge-scrolling list.

use std::time::SystemTime;

use iced_core::keyboard::key::Named;
use iced_core::keyboard::{self, Key};
use iced_core::{Element, Event as IcedEvent, Theme};
use compositor_support_iced_core_engine_base::{IcedUi, Renderer};
use compositor_support_iced_core_engine_base::ui::EventFlags;
use crate::message::LauncherMessage;
use crate::model::{Application, Direction};
use crate::{style, view};
use crate::{search};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Browsing,
    Focused,
}

pub struct Launcher {
    pub(crate) apps: Vec<Application>,
    pub(crate) query: String,
    pub(crate) visible: Vec<usize>,
    /// Cursor's position inside `visible`.
    pub(crate) cursor: usize,
    /// Index into `visible` shown in the leftmost carousel slot.
    /// Invariant when `visible` is non-empty:
    /// `scroll_offset <= cursor < scroll_offset + CAROUSEL_VISIBLE`
    /// (clamped to a valid sub-window of `visible`).
    pub(crate) scroll_offset: usize,
    pub(crate) mode: Mode,
}

impl Launcher {
    pub fn new(apps: Vec<Application>) -> Self {
        let visible = search::rank_default(&apps, SystemTime::now());
        Self {
            apps,
            query: String::new(),
            visible,
            cursor: 0,
            scroll_offset: 0,
            mode: Mode::Browsing,
        }
    }

    // ─── Read-only accessors used by `view` ─────────────────────────

    pub(crate) fn current_title(&self) -> &str {
        self.visible
            .get(self.cursor)
            .map(|&i| self.apps[i].title.as_str())
            .unwrap_or("")
    }

    pub(crate) fn is_focused(&self) -> bool {
        self.mode == Mode::Focused
    }

    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    // ─── Reducer helpers ────────────────────────────────────────────

    /// Re-run search and clamp cursor / scroll. Called whenever the
    /// query or the application list changes.
    fn recompute_visible(&mut self) {
        self.visible = search::search(&self.apps, &self.query, SystemTime::now());
        // Clamp cursor first, then re-derive a sensible scroll_offset.
        if self.cursor >= self.visible.len() {
            self.cursor = self.visible.len().saturating_sub(1);
        }
        self.clamp_scroll();
    }

    /// Ensure the scroll window contains `cursor` and doesn't extend
    /// past the end of `visible`.
    fn clamp_scroll(&mut self) {
        let n = self.visible.len();
        if n == 0 {
            self.scroll_offset = 0;
            return;
        }
        let win = style::CAROUSEL_VISIBLE;

        // If everything fits, anchor to 0.
        if n <= win {
            self.scroll_offset = 0;
            return;
        }

        // Make sure cursor is inside the window.
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor >= self.scroll_offset + win {
            self.scroll_offset = self.cursor + 1 - win;
        }

        // Don't let the window run off the right end.
        let max_offset = n - win;
        if self.scroll_offset > max_offset {
            self.scroll_offset = max_offset;
        }
    }

    /// Move cursor by `delta`. The window scrolls only when the cursor
    /// is at one of the window's edges and would move past it.
    fn move_cursor(&mut self, delta: i32) {
        let n = self.visible.len();
        if n == 0 {
            self.cursor = 0;
            self.scroll_offset = 0;
            return;
        }

        let n_i = n as i32;
        let new_cursor = (self.cursor as i32 + delta).clamp(0, n_i - 1) as usize;
        self.cursor = new_cursor;
        self.clamp_scroll();
    }

    fn set_apps(&mut self, apps: Vec<Application>) {
        self.apps = apps;
        self.query.clear();
        self.mode = Mode::Browsing;
        self.cursor = 0;
        self.scroll_offset = 0;
        self.recompute_visible();
    }

    // ─── Event-process helpers ──────────────────────────────────────

    fn decode_browsing(&self, key: &Key, text: Option<&str>) -> Vec<LauncherMessage> {
        match key {
            Key::Named(Named::ArrowLeft) => vec![LauncherMessage::MoveCursor(-1)],
            Key::Named(Named::ArrowRight) => vec![LauncherMessage::MoveCursor(1)],
            Key::Named(Named::ArrowUp) | Key::Named(Named::ArrowDown) => Vec::new(),
            Key::Named(Named::Enter) => {
                if self.visible.is_empty() {
                    Vec::new()
                } else {
                    vec![LauncherMessage::FocusSelection]
                }
            }
            Key::Named(Named::Escape) => {
                if self.query.is_empty() {
                    vec![LauncherMessage::Exit]
                } else {
                    vec![LauncherMessage::ClearQuery]
                }
            }
            Key::Named(Named::Backspace) => {
                if self.query.is_empty() {
                    Vec::new()
                } else {
                    vec![LauncherMessage::Backspace]
                }
            }
            // IME-friendly text channel (composed key text). Using it
            // instead of parsing `Key::Character` keeps non-Latin
            // layouts and dead-key sequences working correctly.
            _ => {
                let Some(s) = text else { return Vec::new() };
                let filtered: String = s.chars().filter(|c| !c.is_control()).collect();
                if filtered.is_empty() {
                    Vec::new()
                } else {
                    vec![LauncherMessage::AppendText(filtered)]
                }
            }
        }
    }

    fn decode_focused(&self, key: &Key) -> Vec<LauncherMessage> {
        let direction = match key {
            Key::Named(Named::ArrowUp) => Some(Direction::Up),
            Key::Named(Named::ArrowDown) => Some(Direction::Down),
            Key::Named(Named::ArrowLeft) => Some(Direction::Left),
            Key::Named(Named::ArrowRight) => Some(Direction::Right),
            Key::Named(Named::Escape) => {
                return vec![LauncherMessage::UnfocusSelection];
            }
            _ => None,
        };

        let Some(direction) = direction else { return Vec::new() };
        let Some(&idx) = self.visible.get(self.cursor) else { return Vec::new() };

        let app = &self.apps[idx];
        vec![
            LauncherMessage::Launch {
                id: app.id.clone(),
                bin: app.bin.clone(),
                args: app.args.clone(),
                direction,
            },
            // LauncherMessage::Exit,
        ]
    }
}

impl IcedUi for Launcher {
    type Message = LauncherMessage;

    fn subscribe(&self) -> EventFlags {
        EventFlags::KEYBOARD
    }

    fn event_process(&self, event: &IcedEvent) -> Vec<Self::Message> {
        let IcedEvent::Keyboard(keyboard::Event::KeyPressed {
            key, text, ..
        }) = event
        else {
            return Vec::new();
        };

        match self.mode {
            Mode::Browsing => self.decode_browsing(key, text.as_deref()),
            Mode::Focused => self.decode_focused(key),
        }
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            LauncherMessage::Launch { .. } | LauncherMessage::Exit => {}

            LauncherMessage::MoveCursor(delta) => self.move_cursor(delta),

            LauncherMessage::FocusSelection => {
                if !self.visible.is_empty() {
                    self.mode = Mode::Focused;
                }
            }

            LauncherMessage::UnfocusSelection => {
                self.mode = Mode::Browsing;
            }

            LauncherMessage::ClearQuery => {
                self.query.clear();
                self.cursor = 0;
                self.scroll_offset = 0;
                self.recompute_visible();
            }

            LauncherMessage::Backspace => {
                if !self.query.is_empty() {
                    self.query.pop();
                    self.cursor = 0;
                    self.scroll_offset = 0;
                    self.recompute_visible();
                }
            }

            LauncherMessage::AppendText(s) => {
                if !s.is_empty() {
                    self.query.push_str(&s);
                    self.cursor = 0;
                    self.scroll_offset = 0;
                    self.recompute_visible();
                }
            }

            LauncherMessage::Tick => {
                if self.query.is_empty() {
                    self.recompute_visible();
                }
            }

            LauncherMessage::SetApps(apps) => {
                let apps = std::sync::Arc::try_unwrap(apps)
                    .unwrap_or_else(|arc| (*arc).clone());
                self.set_apps(apps);
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message, Theme, Renderer> {
        view::root(self)
    }
}
