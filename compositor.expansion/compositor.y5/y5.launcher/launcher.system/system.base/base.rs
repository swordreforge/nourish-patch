use compositor_support_system_storage_token_base::base::{Token, TokenMut};
use compositor_support_system_trait_system_base::base::{System, WorldBuilder};
use compositor_y5_launcher_draw_state::state::State as LauncherState;

pub static LAUNCHER: Token<LauncherState> = Token::new();
/// TRANSITIONAL pub: legacy call sites still write this slot directly until
/// their logic moves into systems/events (pass 2 of phase 4).
pub static LAUNCHER_MUT: TokenMut<LauncherState> = TokenMut::new(&LAUNCHER);

/// Owns the launcher slot.
#[derive(Default)]
pub struct LauncherSystem;

impl System for LauncherSystem {
    fn name(&self) -> &'static str {
        "launcher"
    }

    fn register(&mut self, builder: &mut WorldBuilder) {
        builder.storage.insert(&LAUNCHER, LauncherState::new());
    }
}
