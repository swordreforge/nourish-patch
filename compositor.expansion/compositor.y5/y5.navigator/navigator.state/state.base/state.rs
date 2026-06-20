use compositor_support_system_storage_token_base::base::{Token, TokenMut};

/// World-storage tokens for the navigator slot (declared beside the type so
/// both the navigator system and readers like the camera system can import
/// them without a system-crate cycle).
pub static NAVIGATOR: Token<Machine> = Token::new();
/// TRANSITIONAL pub: legacy interface fns still drive this slot directly.
pub static NAVIGATOR_MUT: TokenMut<Machine> = TokenMut::new(&NAVIGATOR);

use compositor_y5_navigator_lock_state::state::NavigatorLock;
use std::time::Instant;
use compositor_y5_navigator_travel_state::state::Travel;

/// Eased values the navigator produced THIS tick; the camera system pulls
/// them (same frame — navigator updates before camera) and applies them to
/// its own slot. Cleared by the navigator when idle.
#[derive(Clone, Copy, Debug, Default)]
pub struct NavigatorOutput {
    pub position: Option<(f64, f64)>,
    pub zoom: Option<f64>,
}

// Separate struct for behaviour-less state
// Cant require Loop crate.
#[derive(Default)]
pub struct Machine {
    state: State,
    /// See [`NavigatorOutput`].
    pub output: Option<NavigatorOutput>,
}

#[derive(Default, Clone)]
pub enum State {
    #[default]
    Idle,
    Travel(Travel),
    Lock(compositor_y5_navigator_lock_state::state::NavigatorLock),
}

/// Navigator mutation intent. The interface computes the target (reading space
/// — that legitimately stays rim) then announces this; NavigatorSystem applies
/// it through its buffer, so the rim no longer writes the navigator slot.
#[derive(Clone)]
pub enum NavRequest {
    /// Normal `.set()` transition (ignored while locked).
    Set(State),
    /// Enter lock mode.
    Lock(NavigatorLock),
    /// Leave lock mode.
    Unlock,
}

// navigator.state owns/sends the request channel; NavigatorSystem receives it.
// Senders call `request()` so the pub(crate) TX stays internal here.
compositor_support_system_channel_token_base::y5_channel!(pub NAV_REQUEST, NAV_REQUEST_TX: NavRequest);

/// Announce a navigator request on a world's channel router (rim triggers).
pub fn request(channels: &mut compositor_support_system_channel_router_base::base::ChannelRouter, req: NavRequest) {
    channels.send(&NAV_REQUEST_TX, req);
}

impl Machine {
    // Move implementation to machine instead ( with access to store )
    pub fn set(&mut self, state: State) {
        match self.state {
            State::Lock(_) => {
                return;
            }
            _ => {
                self.state = state;
            }
        }
    }

    /// System-internal transition: unlike `set`, may leave Lock state (used
    /// by the navigator system applying its own tick results).
    pub fn force_set(&mut self, state: State) {
        self.state = state;
    }

    pub fn lock(&mut self, state: NavigatorLock) {
        self.state = State::Lock(state);
    }

    pub fn unlock(&mut self) {
        self.state = State::Idle
    }

    pub fn state(&self) -> &State {
        return &self.state;
    }
}
