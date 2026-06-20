use compositor_support_system_storage_token_base::base::{Token, TokenMut};
use compositor_y5_graphic_capture_registry::CaptureRegistry;
use compositor_y5_graphic_capture_session::session::CaptureState;

/// Capture driver data: the capture registry + session state live in the
/// kernel/driver storage by token, not as Orchestrator fields.
pub static CAPTURE_REGISTRY: Token<Option<CaptureRegistry>> = Token::new();
pub static CAPTURE_REGISTRY_MUT: TokenMut<Option<CaptureRegistry>> = TokenMut::new(&CAPTURE_REGISTRY);
pub static CAPTURE: Token<CaptureState> = Token::new();
pub static CAPTURE_MUT: TokenMut<CaptureState> = TokenMut::new(&CAPTURE);
