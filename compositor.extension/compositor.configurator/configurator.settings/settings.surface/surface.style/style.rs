//! Shared game-like styling for the settings window: a near-solid full-screen
//! backdrop, a right-anchored panel, accent tab buttons, and flat action buttons.
use iced_core::{Background, Border, Color, Theme};
use iced_widget::button::{Status, Style as Button};
use iced_widget::container::Style as Panel;

const ACCENT: Color = Color { r: 0.16, g: 0.45, b: 0.95, a: 1.0 };
const TEXT: Color = Color { r: 0.90, g: 0.93, b: 0.97, a: 1.0 };

fn fill(c: Color) -> Option<Background> {
    Some(Background::Color(c))
}

/// Full-screen dim behind the panel (near-solid so the desktop reads as paused).
pub fn backdrop(_t: &Theme) -> Panel {
    Panel {
        background: fill(Color { r: 0.02, g: 0.03, b: 0.05, a: 0.92 }),
        text_color: Some(TEXT),
        ..Default::default()
    }
}

/// The right-edge settings panel (full height, solid).
pub fn panel(_t: &Theme) -> Panel {
    Panel {
        background: fill(Color { r: 0.09, g: 0.10, b: 0.13, a: 1.0 }),
        text_color: Some(TEXT),
        border: Border { color: Color { r: 0.4, g: 0.6, b: 1.0, a: 0.18 }, width: 1.0, radius: 0.0.into() },
        ..Default::default()
    }
}

/// A tab button — accent-filled when active, faint when idle.
pub fn tab(active: bool) -> impl Fn(&Theme, Status) -> Button {
    move |_t, _s| Button {
        background: fill(if active { ACCENT } else { Color { r: 1.0, g: 1.0, b: 1.0, a: 0.04 } }),
        text_color: if active { Color::WHITE } else { Color { r: 0.66, g: 0.71, b: 0.80, a: 1.0 } },
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 8.0.into() },
        ..Default::default()
    }
}

/// A flat action / field button.
pub fn action(_t: &Theme, _s: Status) -> Button {
    Button {
        background: fill(Color { r: 1.0, g: 1.0, b: 1.0, a: 0.05 }),
        text_color: Color { r: 0.86, g: 0.89, b: 0.94, a: 1.0 },
        border: Border { color: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.10 }, width: 1.0, radius: 7.0.into() },
        ..Default::default()
    }
}

/// The primary accent button (Keep).
pub fn accent(_t: &Theme, _s: Status) -> Button {
    Button {
        background: fill(ACCENT),
        text_color: Color::WHITE,
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 7.0.into() },
        ..Default::default()
    }
}
