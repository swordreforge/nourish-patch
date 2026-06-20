use compositor_introspection_extraction_window_hints_attribute::attribute::{HintAttribute, HintItem};
use compositor_introspection_extraction_window_hints_id::category::AttributeCategory;
use compositor_introspection_extraction_window_hints_source::source::{Confidence, HintSource};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

/// Type-erased hint/preference value (matches `HintItem::value`).
pub type AnyArc = Arc<dyn Any + Send + Sync>;

/// JSON codec + static identity for one hint attribute, so a name-keyed
/// persistence layer can erase ↔ `serde_json::Value` AND rebuild a full
/// `HintItem` (marker `TypeId` + category) without static type info.
pub struct HintCodec {
    pub attr_name: &'static str,
    pub attr_type_id: TypeId,
    pub category: AttributeCategory,
    pub value_type_id: TypeId,
    pub to_json: fn(&AnyArc) -> Option<serde_json::Value>,
    pub from_json: fn(&serde_json::Value) -> Option<AnyArc>,
}

static REGISTRY: OnceLock<RwLock<HashMap<&'static str, HintCodec>>> = OnceLock::new();

fn registry() -> &'static RwLock<HashMap<&'static str, HintCodec>> {
    REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Register a JSON codec for attribute `A` (its `Value` is plain serde). The
/// closures are non-capturing, so they coerce to `fn` per monomorphised `A` —
/// the same fn-pointer trick the persist macros use. Idempotent by name.
pub fn register<A: HintAttribute>()
where
    A::Value: Serialize + DeserializeOwned,
{
    register_raw(HintCodec {
        attr_name: A::name(),
        attr_type_id: TypeId::of::<A>(),
        category: A::category(),
        value_type_id: TypeId::of::<A::Value>(),
        to_json: |arc| arc.downcast_ref::<A::Value>().and_then(|v| serde_json::to_value(v).ok()),
        from_json: |val| {
            serde_json::from_value::<A::Value>(val.clone()).ok().map(|v| Arc::new(v) as AnyArc)
        },
    });
}

/// Register a hand-built codec (for a value without plain serde, e.g. `HandlerId`
/// encoded as a stable string).
pub fn register_raw(codec: HintCodec) {
    registry().write().expect("hint codec registry").insert(codec.attr_name, codec);
}

/// Erase → JSON for attribute `name`, or `None` if unregistered / wrong type.
pub fn encode(name: &str, value: &AnyArc) -> Option<serde_json::Value> {
    let reg = registry().read().expect("hint codec registry");
    (reg.get(name)?.to_json)(value)
}

/// JSON → erase for attribute `name` (raw value), or `None` if unregistered.
pub fn decode(name: &str, value: &serde_json::Value) -> Option<AnyArc> {
    let reg = registry().read().expect("hint codec registry");
    (reg.get(name)?.from_json)(value)
}

/// JSON → a fully-formed `HintItem` (marker id + category restored), for
/// rebuilding an `InferredHints` from persisted rows.
pub fn make_hint_item(
    name: &str,
    value: &serde_json::Value,
    source: HintSource,
    confidence: Confidence,
) -> Option<HintItem> {
    let reg = registry().read().expect("hint codec registry");
    let c = reg.get(name)?;
    let arc = (c.from_json)(value)?;
    Some(HintItem {
        attr_type_id: c.attr_type_id,
        attr_name: c.attr_name,
        attr_category: c.category,
        value: arc,
        source,
        confidence,
    })
}

/// The registered value `TypeId` for `name` (for `Preferences` reconstruction).
pub fn value_type_id(name: &str) -> Option<TypeId> {
    registry().read().expect("hint codec registry").get(name).map(|c| c.value_type_id)
}

/// The `&'static str` attribute name for `name`, needed because `Preferences`
/// keys are `&'static str` (the persisted name is an owned `String`).
pub fn static_name(name: &str) -> Option<&'static str> {
    registry().read().expect("hint codec registry").get(name).map(|c| c.attr_name)
}
