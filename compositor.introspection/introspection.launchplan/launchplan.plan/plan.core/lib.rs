//! LaunchPlan core: the plan type plus the synthesizer trait/registry.
pub mod plan;

pub mod synthesizer {
    //! [`LaunchSynthesizer`]: per-handler command synthesis.
    use crate::plan::LaunchPlan;
    use compositor_introspection_extraction_window_base::HandlerId;
    use std::{collections::HashMap, process::Command, sync::Arc};

    /// Build a runnable `Command` from a `LaunchPlan`. Return `None` to
    /// decline; the plan falls back to generic exec.
    pub trait LaunchSynthesizer: Send + Sync {
        fn handler_id(&self) -> HandlerId;
        fn synthesize(&self, plan: &LaunchPlan) -> Option<Command>;
    }

    /// Registry of synthesizers, keyed by [`HandlerId`].
    #[derive(Default)]
    pub struct SynthesizerRegistry {
        synthesizers: HashMap<HandlerId, Arc<dyn LaunchSynthesizer>>,
    }

    impl SynthesizerRegistry {
        pub fn new() -> Self { Self::default() }
        pub fn register<S: LaunchSynthesizer + 'static>(&mut self, synthesizer: S) -> &mut Self {
            let id = synthesizer.handler_id();
            self.synthesizers.insert(id, Arc::new(synthesizer));
            self
        }
        pub fn get(&self, id: HandlerId) -> Option<&dyn LaunchSynthesizer> { self.synthesizers.get(&id).map(|a| a.as_ref()) }
        pub fn ids(&self) -> impl Iterator<Item = HandlerId> + '_ { self.synthesizers.keys().copied() }
        pub fn len(&self) -> usize { self.synthesizers.len() }
        pub fn is_empty(&self) -> bool { self.synthesizers.is_empty() }
    }
}
