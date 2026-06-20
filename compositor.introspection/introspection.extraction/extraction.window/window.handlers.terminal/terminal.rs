pub use compositor_introspection_extraction_window_handlers_terminal_attributes::attributes;
pub use compositor_introspection_extraction_window_handlers_terminal_id::id::{id, Terminal};

use compositor_introspection_extraction_window_handler_registry::registry::HandlerRegistry;
use compositor_introspection_extraction_window_handler_traits::traits::{AppHandler, DetectResult};
use compositor_introspection_extraction_window_handlers_terminal_extract::extract;
use compositor_introspection_extraction_window_handlers_terminal_flags::flags;
use compositor_introspection_extraction_window_hints_descriptor::descriptor::AttributeDescriptor;
use compositor_introspection_extraction_window_hints_id::handler_id::HandlerId;
use compositor_introspection_extraction_window_hints_inferred::inferred::InferredHints;
use compositor_introspection_extraction_window_hints_source::source::Confidence;
use compositor_introspection_extraction_window_meta_types::types::MetaNode;

use attributes::TerminalKind;

pub struct TerminalHandler;

impl AppHandler for TerminalHandler {
    fn id(&self) -> HandlerId {
        id()
    }

    fn detect(&self, node: &MetaNode) -> DetectResult {
        let kind = node
            .meta
            .app_id
            .as_deref()
            .map(extract::terminal_kind_from_app_id)
            .unwrap_or(TerminalKind::Unknown(String::new()));
        if matches!(kind, TerminalKind::Unknown(_)) {
            DetectResult::miss()
        } else {
            DetectResult::hit(Confidence::High)
        }
    }

    fn extract_hints(&self, node: &MetaNode, hints: &mut InferredHints) {
        let meta = &node.meta;
        extract::push_kind(meta, hints);
        extract::push_launch_cwd(meta, hints);
        extract::push_shell_children(node, hints);
        flags::push_cwd_flags(meta, hints);
    }

    fn attribute_descriptors(&self) -> Vec<AttributeDescriptor> {
        attributes::descriptors()
    }
}

pub fn register(registry: &mut HandlerRegistry) {
    registry.register(TerminalHandler);
}
