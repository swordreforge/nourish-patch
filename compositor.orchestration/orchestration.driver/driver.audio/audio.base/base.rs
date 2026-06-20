use compositor_support_system_storage_token_base::base::{Token, TokenMut};
use compositor_y5_audio_controller_interface::interface::AudioController;
use compositor_y5_audio_controller_interface::media::MediaController;

/// Audio driver data: the audio + media controllers live in the kernel/driver
/// storage (reached by token), not as Orchestrator fields. `Option` because
/// device init can fail. Mutable: systems/handlers drive the controllers.
pub static AUDIO: Token<Option<AudioController>> = Token::new();
pub static AUDIO_MUT: TokenMut<Option<AudioController>> = TokenMut::new(&AUDIO);
pub static MEDIA: Token<Option<MediaController>> = Token::new();
pub static MEDIA_MUT: TokenMut<Option<MediaController>> = TokenMut::new(&MEDIA);
