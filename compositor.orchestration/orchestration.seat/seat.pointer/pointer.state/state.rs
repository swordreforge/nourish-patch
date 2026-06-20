use std::sync::Arc;

use smithay::input::pointer::CursorIcon;
use smithay::utils::{Logical, Point};
use compositor_orchestration_seat_pointer_element::element::PointerElement;
use compositor_orchestration_seat_pointer_texture::pointer_load::CursorThemeCache;

pub struct PointerState {
    pub motion: Point<f64, Logical>,
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
            element: pointer_element,
        };
    }
}
