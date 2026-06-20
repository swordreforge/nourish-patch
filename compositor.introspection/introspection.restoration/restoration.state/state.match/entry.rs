//! [`match_window`]: the pure matching entry point.

use uuid::Uuid;
use compositor_introspection_extraction_window_base::{InferredHints, MetaNode};
use compositor_introspection_restoration_state_matcher::matcher::MatchResult;
use compositor_introspection_restoration_state_pending::pending::PendingRestoration;
use compositor_introspection_restoration_state_registry::registry::MatcherRegistry;

/// Decide which pending restoration (if any) a newly-appeared window
/// satisfies. Pure function.
///
/// Iterates `pendings` in order (FIFO). For each, dispatches to the
/// matcher registered for `pending.plan.active_handler`, falling back
/// to the registry's generic matcher if none is registered. First
/// [`MatchResult::Yes`] wins; the matched pending's id is returned.
///
/// Inputs:
/// - `pendings`: in-flight launches the compositor is waiting on.
/// - `candidate`: the new window's captured metadata.
/// - `candidate_hints`: hints inferred for the new window (matchers may
///   use them for handler-specific signals like terminal kind).
/// - `candidate_token`: the activation token the surface received via
///   Wayland's xdg-activation protocol (set by the compositor from
///   `request_activation`). `None` if the surface didn't carry one.
/// - `matchers`: the per-handler matcher registry.
pub fn match_window(
    pendings: &[PendingRestoration],
    candidate: &MetaNode,
    candidate_hints: &InferredHints,
    candidate_token: Option<&str>,
    matchers: &MatcherRegistry,
) -> Option<Uuid> {
    for pending in pendings {
        let matcher = pending
            .plan
            .active_handler
            .and_then(|id| matchers.get(id))
            .or_else(|| matchers.fallback());

        let Some(matcher) = matcher else { continue };

        match matcher.matches(pending, candidate, candidate_hints, candidate_token) {
            MatchResult::Yes => return Some(pending.id),
            MatchResult::No => continue,
        }
    }
    None
}
