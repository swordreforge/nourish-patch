use compositor_introspection_extraction_window_handler_registry::registry::HandlerRegistry;
use compositor_introspection_extraction_window_handler_traits::traits::{AppHandler, DetectResult};
use compositor_introspection_extraction_window_hints_descriptor::descriptor::AttributeDescriptor;
use compositor_introspection_extraction_window_hints_id::handler_id::HandlerId;
use compositor_introspection_extraction_window_hints_inferred::inferred::InferredHints;
use compositor_introspection_extraction_window_hints_source::source::{Confidence, SourceMethod};
use compositor_introspection_extraction_window_meta_types::types::MetaNode;

use self::attributes::LocationUri;

/// Nautilus-scoped hint attributes.
pub mod attributes {
    use super::id;
    use compositor_introspection_extraction_window_hints_attribute::attribute::HintAttribute;
    use compositor_introspection_extraction_window_hints_descriptor::descriptor::{AttributeDescriptor, AttributeKind};
    use compositor_introspection_extraction_window_hints_id::category::AttributeCategory;

    #[derive(Debug)]
    pub struct LocationUri;
    impl HintAttribute for LocationUri {
        type Value = String;
        fn name() -> &'static str { "nautilus.location_uri" }
        fn category() -> AttributeCategory { AttributeCategory::HandlerScoped(id()) }
        fn descriptor() -> AttributeDescriptor {
            AttributeDescriptor::new(Self::name(), "Location URI", Self::category(), AttributeKind::Text)
        }
    }

    /// All Nautilus-scoped attribute descriptors, in display order.
    pub fn descriptors() -> Vec<AttributeDescriptor> {
        vec![LocationUri::descriptor()]
    }
}

/// Marker type for [`HandlerId::of`].
pub struct Nautilus;

pub fn id() -> HandlerId {
    HandlerId::of::<Nautilus>()
}

pub struct NautilusHandler;

impl AppHandler for NautilusHandler {
    fn id(&self) -> HandlerId {
        id()
    }

    fn detect(&self, node: &MetaNode) -> DetectResult {
        let meta = &node.meta;
        let app_id_match = meta.app_id.as_deref() == Some("org.gnome.Nautilus");
        let exe_match =
            meta.exe.as_ref().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some("nautilus");
        match (app_id_match, exe_match) {
            (true, _) => DetectResult::hit(Confidence::High),
            (false, true) => DetectResult::hit(Confidence::Medium),
            _ => DetectResult::miss(),
        }
    }

    fn extract_hints(&self, node: &MetaNode, hints: &mut InferredHints) {
        // Positional URI argument (rare for Nautilus but possible). v0: no
        // AT-SPI integration; the active location isn't otherwise inferable.
        let Some(cmdline) = &node.meta.cmdline else { return };
        for arg in cmdline.iter().skip(1) {
            if arg.starts_with('/') || arg.starts_with("file://") {
                let uri = if arg.starts_with("file://") {
                    arg.clone()
                } else {
                    format!("file://{arg}")
                };
                hints.push::<LocationUri>(uri, SourceMethod::ProcCmdline, "positional URI/path argument", Confidence::High);
                break;
            }
        }
    }

    fn attribute_descriptors(&self) -> Vec<AttributeDescriptor> {
        attributes::descriptors()
    }
}

pub fn register(registry: &mut HandlerRegistry) {
    registry.register(NautilusHandler);
}
