//! # compositor_introspection_inference_hint_base
//!
//! Façade: the unified inference view. Holds a [`MetaNode`] and its
//! [`InferredHints`] together, exposing typed queries for attributes.

pub mod application_data {
    pub use compositor_introspection_inference_hint_data::application_data::*;
}
pub mod descriptors {
    pub use compositor_introspection_inference_hint_descriptors::descriptors::*;
}

pub use application_data::ApplicationData;
pub use descriptors::{all_descriptors_for, identity_descriptors, launch_descriptors};

// Convenience re-exports — callers usually want these together.
pub use compositor_introspection_extraction_window_base::{
    attributes, AppHandler, AttributeCategory, AttributeDescriptor, AttributeKind,
    Confidence, HandlerId, HandlerRegistry, HintAttribute, HintSource,
    InferredHints, MetaNode, SourceMethod, TypedHint,
};
