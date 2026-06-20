//! [`MatcherRegistry`]: keyed by [`HandlerId`].

use std::collections::HashMap;
use std::sync::Arc;

use compositor_introspection_extraction_window_base::HandlerId;

use compositor_introspection_restoration_state_matcher::matcher::RestorationMatcher;

/// Registry of restoration matchers.
///
/// One matcher per [`HandlerId`]. A fallback matcher (typically the
/// generic one) handles handlers without specific matchers and any
/// restoration where `active_handler` is `None`.
///
/// Same shape as `compositor_introspection_launchplan_plan_base::SynthesizerRegistry`.
#[derive(Default)]
pub struct MatcherRegistry {
    matchers: HashMap<HandlerId, Arc<dyn RestorationMatcher>>,
    fallback: Option<Arc<dyn RestorationMatcher>>,
}

impl MatcherRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<M: RestorationMatcher + 'static>(&mut self, matcher: M) -> &mut Self {
        let id = matcher.handler_id();
        self.matchers.insert(id, Arc::new(matcher));
        self
    }

    /// Mark a matcher as the fallback. The matcher must implement
    /// `RestorationMatcher`; its `handler_id()` value is preserved for
    /// inspection but isn't used for fallback dispatch.
    pub fn set_fallback<M: RestorationMatcher + 'static>(&mut self, matcher: M) -> &mut Self {
        self.fallback = Some(Arc::new(matcher));
        self
    }

    pub fn get(&self, id: HandlerId) -> Option<&dyn RestorationMatcher> {
        self.matchers.get(&id).map(|a| a.as_ref())
    }

    pub fn fallback(&self) -> Option<&dyn RestorationMatcher> {
        self.fallback.as_ref().map(|a| a.as_ref())
    }

    pub fn len(&self) -> usize {
        self.matchers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.matchers.is_empty()
    }
}
