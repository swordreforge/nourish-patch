use compositor_introspection_extraction_window_hints_descriptor::descriptor::AttributeDescriptor;
use compositor_introspection_extraction_window_hints_id::handler_id::HandlerId;
use compositor_introspection_extraction_window_hints_inferred::inferred::InferredHints;
use compositor_introspection_extraction_window_hints_source::source::Confidence;
use compositor_introspection_extraction_window_meta_types::types::MetaNode;

/// What `AppHandler::detect` returns.
#[derive(Debug, Clone)]
pub struct DetectResult {
    pub matches: bool,
    pub confidence: Confidence,
}

impl DetectResult {
    pub fn miss() -> Self {
        Self { matches: false, confidence: Confidence::Low }
    }
    pub fn hit(confidence: Confidence) -> Self {
        Self { matches: true, confidence }
    }
}

/// One handler. Responsible for detection (claiming a window), hint
/// extraction (handler-specific attributes) and descriptor enumeration.
///
/// Synthesis (turning hints + preferences into a `Command`) lives in the
/// `compositor_introspection_launchplan_plan_base` crate, keeping this layer
/// free of any dependency on plans, preferences, or process spawning.
pub trait AppHandler: Send + Sync {
    /// Identity of this handler. Stable; same instance always returns the
    /// same id.
    fn id(&self) -> HandlerId;

    /// Whether this handler claims the given window, and how confidently.
    fn detect(&self, node: &MetaNode) -> DetectResult;

    /// Push handler-specific hints derived from inspecting `node`.
    ///
    /// The base hints (display name, exec, icon, etc.) are already in
    /// `hints` from the registry's base extractor. This method adds the
    /// handler's own attribute markers — chrome.profile_directory,
    /// jetbrains.project_path, etc.
    fn extract_hints(&self, _node: &MetaNode, _hints: &mut InferredHints) {}

    /// All handler-scoped attribute descriptors, in display order.
    ///
    /// Used by higher crates to enumerate the editor sections shown for
    /// this handler. Default: empty (Generic and similar handlers have
    /// no handler-specific attributes).
    fn attribute_descriptors(&self) -> Vec<AttributeDescriptor> {
        Vec::new()
    }
}
