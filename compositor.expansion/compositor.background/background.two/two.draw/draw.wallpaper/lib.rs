//! Tile rendering: upload decoded tile bytes to GPU and blit visible tiles
//! into the current render target.
//!
//! Consumed by `draw.parallax` — `ParallaxBackground::draw()` delegates to
//! this module when a wallpaper path is configured.

mod wallpaper;
pub use wallpaper::*;
