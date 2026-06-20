use compositor_support_system_storage_token_base::base::{Token, TokenMut};
use compositor_support_system_trait_system_base::base::{System, WorldBuilder};
use compositor_orchestration_seat_pointer_state::state::PointerState;

pub static POINTER: Token<PointerState> = Token::new();
/// TRANSITIONAL pub: legacy call sites still write this slot directly until
/// their logic moves into systems/events (pass 2 of phase 4).
pub static POINTER_MUT: TokenMut<PointerState> = TokenMut::new(&POINTER);

/// Owns the pointer slot.
#[derive(Default)]
pub struct PointerSystem;

impl System for PointerSystem {
    fn name(&self) -> &'static str {
        "pointer"
    }

    fn register(&mut self, builder: &mut WorldBuilder) {
        builder.storage.insert(&POINTER, PointerState::new());
    }
}
