//! Always matches with Low confidence. Has no handler-specific attributes.
//! Used as the registry fallback so every window has at least a handler
//! that claims it.

use compositor_introspection_extraction_window_handler_registry::registry::HandlerRegistry;
use compositor_introspection_extraction_window_handler_traits::traits::{AppHandler, DetectResult};
use compositor_introspection_extraction_window_hints_id::handler_id::HandlerId;
use compositor_introspection_extraction_window_hints_source::source::Confidence;
use compositor_introspection_extraction_window_meta_types::types::MetaNode;

/// Marker type for [`HandlerId::of`].
pub struct Generic;

pub fn id() -> HandlerId {
    HandlerId::of::<Generic>()
}

pub struct GenericHandler;

impl AppHandler for GenericHandler {
    fn id(&self) -> HandlerId {
        id()
    }

    fn detect(&self, _node: &MetaNode) -> DetectResult {
        DetectResult::hit(Confidence::Low)
    }

    // No extract_hints override — base hints already cover everything generic.
}

/// Register this handler and set it as the registry's fallback.
pub fn register(registry: &mut HandlerRegistry) {
    registry.register(GenericHandler);
    registry.set_fallback(id());
}
