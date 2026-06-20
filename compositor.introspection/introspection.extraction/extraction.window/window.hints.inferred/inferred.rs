use compositor_introspection_extraction_window_hints_attribute::attribute::{HintAttribute, HintItem, TypedHint};
use compositor_introspection_extraction_window_hints_inferred_raw::inferred::{self as raw, RawAlternative, RawHintView};
use compositor_introspection_extraction_window_hints_source::source::{Confidence, HintSource, SourceMethod};
use std::any::TypeId;
use std::fmt;

/// Internally a flat vector of typed items. Multiple hints for the same
/// attribute coexist; queries return all of them, and helpers pick the
/// highest-confidence one when a single value is needed.
#[derive(Clone, Default)]
pub struct InferredHints {
    pub items: Vec<HintItem>,
}

impl InferredHints {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Push a hint; multiple pushes of the same attribute are all retained.
    pub fn push<A: HintAttribute>(&mut self, value: A::Value, method: SourceMethod, detail: impl Into<String>, confidence: Confidence) {
        self.items
            .push(HintItem::new::<A>(value, HintSource::new(method, detail), confidence));
    }

    /// Push a hint with a pre-built source (copying source info across attrs).
    pub fn push_with_source<A: HintAttribute>(&mut self, value: A::Value, source: HintSource, confidence: Confidence) {
        self.items.push(HintItem {
            attr_type_id: TypeId::of::<A>(),
            attr_name: A::name(),
            attr_category: A::category(),
            value: std::sync::Arc::new(value),
            source,
            confidence,
        });
    }

    /// All hints for this attribute, in insertion order.
    pub fn get<A: HintAttribute>(&self) -> Vec<TypedHint<A::Value>> {
        self.items.iter().filter_map(|i| i.typed::<A>()).collect()
    }

    /// Highest-confidence hint, ties broken by insertion order (earliest wins).
    pub fn best<A: HintAttribute>(&self) -> Option<TypedHint<A::Value>> {
        let mut best: Option<&HintItem> = None;
        for item in &self.items {
            if item.attr_type_id != TypeId::of::<A>() {
                continue;
            }
            match &best {
                None => best = Some(item),
                Some(cur) if item.confidence.rank() > cur.confidence.rank() => best = Some(item),
                _ => {}
            }
        }
        best.and_then(|item| item.typed::<A>())
    }

    /// Convenience: the best hint's value, dropping source/confidence.
    pub fn best_value<A: HintAttribute>(&self) -> Option<A::Value> {
        self.best::<A>().map(|h| h.value)
    }

    /// Like `best` but keyed by attribute name string; type-erased. Used by
    /// UI layers that iterate over descriptors without static type info.
    pub fn best_raw(&self, name: &str) -> Option<std::sync::Arc<dyn std::any::Any + Send + Sync>> {
        raw::best_raw(&self.items, name)
    }

    /// True if at least one hint exists for this attribute.
    pub fn has<A: HintAttribute>(&self) -> bool {
        self.items.iter().any(|i| i.attr_type_id == TypeId::of::<A>())
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Untyped views over the raw items (debug/inspection + UI listing).
    pub fn iter_raw(&self) -> impl Iterator<Item = RawHintView<'_>> {
        raw::iter_raw(&self.items)
    }

    /// All hints for an attribute by name, type-erased, insertion order.
    pub fn available_raw(&self, name: &str) -> Vec<RawAlternative> {
        raw::available_raw(&self.items, name)
    }
}

impl fmt::Debug for InferredHints {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        raw::fmt_items(&self.items, f)
    }
}
