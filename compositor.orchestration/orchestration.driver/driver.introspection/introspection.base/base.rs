use compositor_support_system_storage_token_base::base::{Token, TokenMut};
use compositor_introspection_sampler_window_base::sampler::Sampler;

/// Introspection driver data: the window sampler lives in the kernel/driver
/// storage by token, not as an Orchestrator field. `Option` — populated post-init
/// by the loader once the sampler thread is spawned.
pub static SAMPLER: Token<Option<Sampler>> = Token::new();
pub static SAMPLER_MUT: TokenMut<Option<Sampler>> = TokenMut::new(&SAMPLER);
