//! Activation-token comparison helpers shared by matchers.

use compositor_introspection_extraction_window_base::MetaNode;

use compositor_introspection_restoration_state_pending::pending::PendingRestoration;

/// Env var name for the XDG activation token. Set by the compositor on
/// the launched process; carried into the new window's `/proc/<pid>/environ`
/// (allowlisted in the extraction crate's `ENV_ALLOWLIST`).
pub const ACTIVATION_TOKEN_ENV: &str = "XDG_ACTIVATION_TOKEN";

/// Legacy startup-notification env var. Some clients consume
/// `XDG_ACTIVATION_TOKEN` and unset it but leave `DESKTOP_STARTUP_ID`
/// behind, or vice-versa. We check both.
pub const STARTUP_ID_ENV: &str = "DESKTOP_STARTUP_ID";

/// Read the activation token from the candidate's allowlisted env, if
/// present. Tries `XDG_ACTIVATION_TOKEN` first, falls back to
/// `DESKTOP_STARTUP_ID`.
pub fn candidate_token_from_env(candidate: &MetaNode) -> Option<&str> {
    let env = candidate.meta.selected_env.as_ref()?;
    env.get(ACTIVATION_TOKEN_ENV)
        .or_else(|| env.get(STARTUP_ID_ENV))
        .map(String::as_str)
}

/// True if the candidate's activation token (from surface data OR env)
/// matches the pending restoration's stored token.
///
/// Three sources are checked in order:
/// 1. `candidate_token` — the surface-data token set by the compositor
///    from `request_activation`. This is the protocol-level signal.
/// 2. `XDG_ACTIVATION_TOKEN` in the candidate's allowlisted env. The
///    last-resort signal for clients that received the token but never
///    called `xdg_activation_v1.activate`.
/// 3. `DESKTOP_STARTUP_ID` in the candidate's allowlisted env. Same
///    role for legacy clients (and apps that strip one but not the other).
///
/// The pending's stored token is checked under both env-var names so
/// either side of the asymmetry works.
pub fn token_matches(
    pending: &PendingRestoration,
    candidate: &MetaNode,
    candidate_token: Option<&str>,
) -> bool {
    // What tokens does the pending know about? Usually the same value
    // is set under both var names, but allow either.
    let pending_xdg = pending.activation_env.get(ACTIVATION_TOKEN_ENV);
    let pending_startup = pending.activation_env.get(STARTUP_ID_ENV);

    let pending_tokens: [Option<&String>; 2] = [pending_xdg, pending_startup];
    let pending_tokens: Vec<&str> = pending_tokens
        .into_iter()
        .flatten()
        .map(String::as_str)
        .collect();
    if pending_tokens.is_empty() {
        return false;
    }

    // Source 1: surface-data token.
    if let Some(t) = candidate_token {
        if pending_tokens.iter().any(|p| *p == t) {
            return true;
        }
    }

    // Sources 2 & 3: candidate env.
    if let Some(env) = candidate.meta.selected_env.as_ref() {
        if let Some(t) = env.get(ACTIVATION_TOKEN_ENV) {
            if pending_tokens.iter().any(|p| *p == t.as_str()) {
                return true;
            }
        }
        if let Some(t) = env.get(STARTUP_ID_ENV) {
            if pending_tokens.iter().any(|p| *p == t.as_str()) {
                return true;
            }
        }
    }

    false
}
