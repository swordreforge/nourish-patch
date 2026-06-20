use compositor_y5_lock_state_base::state::LockState;
use compositor_support_system_storage_token_base::base::{Token, TokenMut};
use compositor_support_system_trait_system_base::base::{System, WorldBuilder};

/// The lock overlay's fixed world id (single source is `WorldManager`). The
/// session world is resolved dynamically via `WorldManager::spawn_target()` / the
/// Orchestrator focus accessors, never a literal id (document/WORLD_DELEGATION.md).
pub use compositor_orchestration_world_manager_base::manager::LOCK_WORLD;

pub static LOCK: Token<LockState> = Token::new();
/// TRANSITIONAL pub: the legacy lock interface/scene paths still drive this
/// slot directly until they become this world's systems.
pub static LOCK_MUT: TokenMut<LockState> = TokenMut::new(&LOCK);

/// Owns the lock-screen state slot — registered in the LOCK world, not main.
/// Locking is a world switch: WorldManager::switch(LOCK_WORLD) fires
/// on_disable on the session systems and on_enable here.
#[derive(Default)]
pub struct LockSystem;

impl System for LockSystem {
    fn name(&self) -> &'static str {
        "lock"
    }

    fn register(&mut self, builder: &mut WorldBuilder) {
        builder.storage.insert(&LOCK, LockState::new());
    }
}
