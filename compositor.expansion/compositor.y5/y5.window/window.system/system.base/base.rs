use compositor_support_system_storage_token_base::base::{Token, TokenMut};
use compositor_support_system_trait_system_base::base::{System, WorldBuilder};
use compositor_y5_window_lifecycle_state::lifecycle::WindowLifecycle;

pub static WINDOW_LIFECYCLE: Token<WindowLifecycle> = Token::new();
/// TRANSITIONAL pub: the wire glue and lifecycle interface still write the
/// incoming queue directly until they become events (pass 2 of phase 4).
pub static WINDOW_LIFECYCLE_MUT: TokenMut<WindowLifecycle> = TokenMut::new(&WINDOW_LIFECYCLE);

/// Owns the window-lifecycle slot (incoming map/destroy/fullscreen events).
#[derive(Default)]
pub struct WindowSystem;

impl System for WindowSystem {
    fn name(&self) -> &'static str {
        "window"
    }

    fn register(&mut self, builder: &mut WorldBuilder) {
        builder.storage.insert(&WINDOW_LIFECYCLE, WindowLifecycle::new());
    }
}
