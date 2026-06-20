use compositor_introspection_extraction_window_hints_attribute::attribute::HintItem;
use compositor_introspection_extraction_window_hints_id::category::AttributeCategory;
use compositor_introspection_extraction_window_hints_source::source::{Confidence, HintSource};
use std::any::Any;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

/// Type-erased single hint, used by UI layers to render an "alternatives"
/// picker. The `value` Arc can be downcast to the attribute's `Value` type
/// (the UI knows what to expect from the `AttributeDescriptor::kind`).
#[derive(Clone)]
pub struct RawAlternative {
    pub value: Arc<dyn Any + Send + Sync>,
    pub source: HintSource,
    pub confidence: Confidence,
}

impl fmt::Debug for RawAlternative {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawAlternative")
            .field("source", &self.source)
            .field("confidence", &self.confidence)
            .finish()
    }
}

/// Untyped view of a hint item, exposed for debugging / UI listing.
/// Typed accessors give the value back; rendering depends on its type.
#[derive(Debug)]
pub struct RawHintView<'a> {
    pub attr_name: &'static str,
    pub attr_category: AttributeCategory,
    pub source: &'a HintSource,
    pub confidence: Confidence,
}

/// Highest-confidence item with this attribute name, type-erased.
pub fn best_raw(items: &[HintItem], name: &str) -> Option<Arc<dyn Any + Send + Sync>> {
    let mut best: Option<&HintItem> = None;
    for item in items {
        if item.attr_name != name {
            continue;
        }
        match best {
            None => best = Some(item),
            Some(cur) if item.confidence.rank() > cur.confidence.rank() => best = Some(item),
            _ => {}
        }
    }
    best.map(|i| i.value.clone())
}

/// All items with this attribute name, type-erased, in insertion order.
pub fn available_raw(items: &[HintItem], name: &str) -> Vec<RawAlternative> {
    items
        .iter()
        .filter(|item| item.attr_name == name)
        .map(|item| RawAlternative {
            value: item.value.clone(),
            source: item.source.clone(),
            confidence: item.confidence,
        })
        .collect()
}

/// Untyped views over all items.
pub fn iter_raw(items: &[HintItem]) -> impl Iterator<Item = RawHintView<'_>> {
    items.iter().map(|i| RawHintView {
        attr_name: i.attr_name,
        attr_category: i.attr_category,
        source: &i.source,
        confidence: i.confidence,
    })
}

/// Debug-format hint rows grouped by attribute name for readability.
pub fn fmt_items(items: &[HintItem], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut entries: BTreeMap<&'static str, Vec<(&HintSource, Confidence)>> = BTreeMap::new();
    for item in items {
        entries
            .entry(item.attr_name)
            .or_default()
            .push((&item.source, item.confidence));
    }
    f.debug_struct("InferredHints").field("items", &entries).finish()
}
