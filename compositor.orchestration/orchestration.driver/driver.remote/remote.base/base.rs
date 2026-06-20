use compositor_support_system_storage_token_base::base::{Token, TokenMut};
use compositor_remote_message_state_base::state::State;

/// Remote driver data: the RPC state (broadcast sender + incoming buffer) lives
/// in the kernel/driver storage by token, not as an Orchestrator field.
pub static RPC: Token<State> = Token::new();
pub static RPC_MUT: TokenMut<State> = TokenMut::new(&RPC);
