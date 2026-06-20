pub use compositor_introspection_extraction_window_handlers_jetbrains_attributes::attributes;
pub use compositor_introspection_extraction_window_handlers_jetbrains_id::id::{id, JetBrains};

use compositor_introspection_extraction_window_handler_registry::registry::HandlerRegistry;
use compositor_introspection_extraction_window_handler_traits::traits::{AppHandler, DetectResult};
use compositor_introspection_extraction_window_handlers_jetbrains_extract::extract;
use compositor_introspection_extraction_window_hints_descriptor::descriptor::AttributeDescriptor;
use compositor_introspection_extraction_window_hints_id::handler_id::HandlerId;
use compositor_introspection_extraction_window_hints_inferred::inferred::InferredHints;
use compositor_introspection_extraction_window_hints_source::source::Confidence;
use compositor_introspection_extraction_window_meta_types::types::MetaNode;

pub struct JetBrainsHandler;

impl AppHandler for JetBrainsHandler {
    fn id(&self) -> HandlerId {
        id()
    }

    fn detect(&self, node: &MetaNode) -> DetectResult {
        let meta = &node.meta;
        let by_app_id = meta
            .app_id
            .as_deref()
            .map(|s| s.starts_with("jetbrains-") || s.contains("idea") || s.contains("pycharm"))
            .unwrap_or(false);
        let by_exe = meta
            .exe
            .as_ref()
            .and_then(|p| p.to_str())
            .map(|s| s.contains("JetBrains") || s.contains("idea") || s.contains("pycharm"))
            .unwrap_or(false);
        match (by_app_id, by_exe) {
            (true, _) => DetectResult::hit(Confidence::High),
            (false, true) => DetectResult::hit(Confidence::Medium),
            _ => DetectResult::miss(),
        }
    }

    fn extract_hints(&self, node: &MetaNode, hints: &mut InferredHints) {
        let meta = &node.meta;
        extract::push_product(meta, hints);
        extract::push_launcher(meta, hints);
        extract::push_title_guess(meta, hints);
        extract::push_project_path(meta, hints);
    }

    fn attribute_descriptors(&self) -> Vec<AttributeDescriptor> {
        attributes::descriptors()
    }
}

pub fn register(registry: &mut HandlerRegistry) {
    registry.register(JetBrainsHandler);
}
