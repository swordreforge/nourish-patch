use compositor_support_bevy_core_compositor_base::WgpuVulkanContext;
use compositor_background_three_state_base::state::Three;
use compositor_support_system_storage_token_base::base::{Token, TokenMut};
use compositor_support_system_trait_system_base::base::{System, WorldBuilder};
use std::sync::Arc;

pub static BG_THREE: Token<Three> = Token::new();
/// TRANSITIONAL pub: lock scenes still mutate the registry directly (phase 5).
pub static BG_THREE_MUT: TokenMut<Three> = TokenMut::new(&BG_THREE);

/// Shared bevy GPU context — KERNEL driver data, mirroring iced's `ICED_CONTEXT`.
/// The loader block-waits the async wgpu init at startup and stores it here ONCE;
/// each world running ThreeSystem then has its `BevyRegistry` built from it by the
/// startup prewarm pass (`three_system_prewarm::ensure_registry`), off the render
/// path. Capture also reads this context. It is asserted present after startup.
pub static BEVY_CONTEXT: Token<Option<Arc<WgpuVulkanContext>>> = Token::new();
pub static BEVY_CONTEXT_MUT: TokenMut<Option<Arc<WgpuVulkanContext>>> = TokenMut::new(&BEVY_CONTEXT);

/// The bevy 3D background system. The wgpu context lives in the kernel
/// (`BEVY_CONTEXT`); this world's `BevyRegistry` is pre-created from it by the
/// loader's prewarm pass, not lazily during `update()`.
#[derive(Default)]
pub struct ThreeSystem;

impl System for ThreeSystem {
    fn name(&self) -> &'static str {
        "background.three"
    }

    fn register(&mut self, builder: &mut WorldBuilder) {
        builder.storage.insert(&BG_THREE, Three::new());
    }
}
