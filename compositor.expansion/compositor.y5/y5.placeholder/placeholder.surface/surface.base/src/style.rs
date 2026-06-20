//! Visual constants used across the placeholder UI.

use iced_core::{Color, Padding};

// Background tones
pub const BG: Color = Color { r: 0.04, g: 0.05, b: 0.07, a: 1.0 };
pub const PANEL_BG: Color = Color { r: 0.08, g: 0.09, b: 0.12, a: 1.0 };
pub const PANEL_BG_SOFT: Color = Color { r: 0.10, g: 0.12, b: 0.16, a: 0.85 };

// Text
pub const TEXT: Color = Color { r: 0.92, g: 0.94, b: 0.97, a: 1.0 };
pub const TEXT_DIM: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.55 };
pub const TEXT_HINT: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.35 };

// Lines / accents
pub const BORDER: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.10 };
pub const BORDER_BRIGHT: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.22 };
pub const ACCENT: Color = Color { r: 0.40, g: 0.65, b: 0.95, a: 1.0 };
pub const GLOW: Color = Color { r: 0.40, g: 0.65, b: 0.95, a: 0.30 };

// Icon backdrop (icon container in view mode)
pub const ICON_BG: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.06 };
pub const ICON_HIGHLIGHT: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.18 };

// Padding
pub const PAD_SMALL: Padding = Padding { top: 6.0, right: 10.0, bottom: 6.0, left: 10.0 };
pub const PAD_MEDIUM: Padding = Padding { top: 10.0, right: 14.0, bottom: 10.0, left: 14.0 };
pub const PAD_LARGE: Padding = Padding { top: 24.0, right: 24.0, bottom: 24.0, left: 24.0 };
pub const PAD_XLARGE: Padding = Padding { top: 36.0, right: 36.0, bottom: 36.0, left: 36.0 };

// Border radii
pub const RADIUS_SMALL: f32 = 6.0;
pub const RADIUS_MEDIUM: f32 = 10.0;
pub const RADIUS_LARGE: f32 = 16.0;

// Text sizes
pub const TEXT_SIZE_TITLE: f32 = 22.0;
pub const TEXT_SIZE_SECTION: f32 = 16.0;
pub const TEXT_SIZE_BODY: f32 = 14.0;
pub const TEXT_SIZE_HINT: f32 = 12.0;
