use std::sync::Arc;

use smithay::input::pointer::CursorIcon;
use smithay::utils::{Logical, Point};
use compositor_orchestration_seat_pointer_element::element::PointerElement;
use compositor_orchestration_seat_pointer_texture::pointer_load::CursorThemeCache;

pub struct PointerState {
    pub motion: Point<f64, Logical>,
    /// Whether `motion` (the physical-space accumulator) has been seeded to the
    /// drawn cursor position. Its `(0,0)` default is physical top-left, but the
    /// cursor is rendered at the seat's world location (`(0,0)` -> screen center
    /// at boot); without seeding, the first relative delta would accumulate from
    /// the corner and the cursor would jump there. Seeded once when the output
    /// is registered (see `display.output::register`).
    pub initialized: bool,
    pub element: PointerElement,
}

impl PointerState {
    pub fn new() -> PointerState {
        let theme_name = std::env::var("XCURSOR_THEME").unwrap_or_else(|_| "Adwaita".into());
        let size = std::env::var("XCURSOR_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(24);

            
        let theme = Arc::new(CursorThemeCache::new(&theme_name, size));

        let pointer_element = PointerElement::new(theme);

        return PointerState {
            motion: Point::new(0.0, 0.0),
            initialized: false,
            element: pointer_element,
        };
    }
}
