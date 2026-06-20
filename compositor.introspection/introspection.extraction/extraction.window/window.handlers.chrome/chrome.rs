/// Chrome-scoped hint attributes.
pub mod attributes {
    pub use compositor_introspection_extraction_window_handlers_chrome_attributes::attributes::*;
    pub use compositor_introspection_extraction_window_handlers_chrome_attributes_more::attributes::*;
}

pub use compositor_introspection_extraction_window_handlers_chrome_id::id::{id, Chrome};

use compositor_introspection_extraction_window_handler_registry::registry::HandlerRegistry;
use compositor_introspection_extraction_window_handler_traits::traits::{AppHandler, DetectResult};
use compositor_introspection_extraction_window_handlers_chrome_extract::extract;
use compositor_introspection_extraction_window_handlers_chrome_profiles::profiles;
use compositor_introspection_extraction_window_hints_descriptor::descriptor::AttributeDescriptor;
use compositor_introspection_extraction_window_hints_id::handler_id::HandlerId;
use compositor_introspection_extraction_window_hints_inferred::inferred::InferredHints;
use compositor_introspection_extraction_window_hints_source::source::Confidence;
use compositor_introspection_extraction_window_meta_types::types::MetaNode;

pub struct ChromeHandler;

impl AppHandler for ChromeHandler {
    fn id(&self) -> HandlerId {
        id()
    }

    fn detect(&self, node: &MetaNode) -> DetectResult {
        let meta = &node.meta;
        let app_id = meta.app_id.as_deref().unwrap_or("").to_lowercase();
        let by_app_id = app_id == "google-chrome"
            || app_id == "chromium-browser"
            || app_id == "chromium"
            || app_id.contains("chrome");
        let by_exe = meta
            .exe
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .map(|s| matches!(s, "chrome" | "chromium" | "google-chrome-stable" | "brave"))
            .unwrap_or(false);
        match (by_app_id, by_exe) {
            (true, _) => DetectResult::hit(Confidence::High),
            (false, true) => DetectResult::hit(Confidence::Medium),
            _ => DetectResult::miss(),
        }
    }

    fn extract_hints(&self, node: &MetaNode, hints: &mut InferredHints) {
        let meta = &node.meta;
        extract::push_variant(meta, hints);
        extract::push_cmdline_hints(meta, hints);
        extract::push_title_hint(meta, hints);
        profiles::push_available_profiles(hints);
    }

    fn attribute_descriptors(&self) -> Vec<AttributeDescriptor> {
        attributes::descriptors()
    }
}

pub fn register(registry: &mut HandlerRegistry) {
    registry.register(ChromeHandler);
}
