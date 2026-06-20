//! Facade: `ParallaxBackground` lives in the flat sibling crates
//! (`draw.parallax` / `draw.motion` / `draw.program`); the public
//! `element::ParallaxBackground` path keeps resolving. The `shaders/`
//! directory stays here (`draw.program` embeds `shaders/spacev3.frag`).

pub mod element {
    pub use compositor_background_two_draw_parallax::ParallaxBackground;
}
