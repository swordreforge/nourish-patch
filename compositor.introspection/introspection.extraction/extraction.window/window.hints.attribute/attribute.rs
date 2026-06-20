use compositor_introspection_extraction_window_hints_descriptor::descriptor::AttributeDescriptor;
use compositor_introspection_extraction_window_hints_id::category::AttributeCategory;
use compositor_introspection_extraction_window_hints_source::source::{Confidence, HintSource};
use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::sync::Arc;

/// A typed view of one hint: the value plus its source and confidence.
#[derive(Debug, Clone)]
pub struct TypedHint<V: Clone> {
    pub value: V,
    pub source: HintSource,
    pub confidence: Confidence,
}

impl<V: Clone> TypedHint<V> {
    pub fn rank(&self) -> u8 {
        self.confidence.rank()
    }
}

/// Marker trait for hint attribute kinds.
///
/// Each conceptual hint kind (display name, icon path, Chrome profile dir,
/// etc.) is a distinct zero-sized type implementing this trait. The `Value`
/// associated type is what the hint actually holds. External crates can
/// define their own attributes; storage uses TypeId, so no collisions.
pub trait HintAttribute: 'static {
    type Value: Clone + Debug + Send + Sync + 'static;

    /// Stable identifier used as the preference-map key in higher crates.
    /// Convention: `<scope>.<field>`, e.g. `"chrome.profile_directory"`.
    fn name() -> &'static str;

    /// What scope this attribute belongs to.
    fn category() -> AttributeCategory;

    /// UI-level descriptor for this attribute (editor + settings label).
    fn descriptor() -> AttributeDescriptor;
}

/// Storage row in the InferredHints vector. Type-erased.
#[derive(Debug, Clone)]
pub struct HintItem {
    pub attr_type_id: TypeId,
    pub attr_name: &'static str,
    pub attr_category: AttributeCategory,
    pub value: Arc<dyn Any + Send + Sync>,
    pub source: HintSource,
    pub confidence: Confidence,
}

impl HintItem {
    pub fn new<A: HintAttribute>(
        value: A::Value,
        source: HintSource,
        confidence: Confidence,
    ) -> Self {
        Self {
            attr_type_id: TypeId::of::<A>(),
            attr_name: A::name(),
            attr_category: A::category(),
            value: Arc::new(value),
            source,
            confidence,
        }
    }

    pub fn typed<A: HintAttribute>(&self) -> Option<TypedHint<A::Value>> {
        if self.attr_type_id != TypeId::of::<A>() {
            return None;
        }
        let value = self.value.downcast_ref::<A::Value>()?.clone();
        Some(TypedHint {
            value,
            source: self.source.clone(),
            confidence: self.confidence,
        })
    }
}
