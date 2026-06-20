//! [`ApplicationData`]: the inference view.

use compositor_introspection_extraction_window_base::{HintAttribute, InferredHints, MetaNode, TypedHint};

/// The unified inference view: a captured process tree and the hints
/// inferred from it.
///
/// This struct is the "raw inference" — without preferences applied.
/// Higher crates compose it with [`Preferences`] to produce an effective
/// [`LaunchPlan`] (see `compositor_introspection_launchplan_plan_base`).
///
/// ## Re-extraction
///
/// To refresh, call [`compositor_introspection_extraction_window_base::extract_meta`] and
/// [`compositor_introspection_extraction_window_base::extract_hints`] again, then replace
/// `meta` and `hints` in place. There's no merge logic — each extraction
/// is independent.
#[derive(Debug, Clone)]
pub struct ApplicationData {
    pub meta: MetaNode,
    pub hints: InferredHints,
}

impl ApplicationData {
    pub fn new(meta: MetaNode, hints: InferredHints) -> Self {
        Self { meta, hints }
    }

    /// All hints for the given attribute, in insertion order.
    /// Empty if no hints were inferred for it.
    pub fn available<A: HintAttribute>(&self) -> Vec<TypedHint<A::Value>> {
        self.hints.get::<A>()
    }

    /// Highest-confidence hint for the given attribute. Ties broken by
    /// insertion order (earliest wins).
    pub fn best<A: HintAttribute>(&self) -> Option<TypedHint<A::Value>> {
        self.hints.best::<A>()
    }

    /// Same as [`best`](Self::best) but returns just the value.
    pub fn best_value<A: HintAttribute>(&self) -> Option<A::Value> {
        self.hints.best_value::<A>()
    }

    /// True if at least one hint exists for the attribute.
    pub fn has<A: HintAttribute>(&self) -> bool {
        self.hints.has::<A>()
    }
}
