use compositor_support_library_pam_worker_base::zero_string::ZeroString;

#[derive(Clone, Debug)]
pub enum LockMessage {
    /// User typed a character.
    AppendChar(char),
    /// User pressed Backspace.
    Backspace,
    /// User pressed Escape (clear input).
    Clear,
    /// User pressed Enter or clicked the Unlock button.
    /// `process()` translates this into `Attempt` when valid.
    Submit,
    /// Authentication request to be dispatched to the PAM worker by
    /// the compositor. Carries the password wrapped in `ZeroString`
    /// so it is zeroed when this message is dropped.
    Attempt(ZeroString),
    /// Auth result: rejected. Updates the UI error state.
    AuthFailed(String),
    /// Auth result: accepted. Compositor should tear down the lock.
    AuthSucceeded,
}
