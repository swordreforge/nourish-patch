//! JetBrains handler identity marker (split out of window.base `handlers`).
pub mod id {
    use compositor_introspection_extraction_window_hints_id::handler_id::HandlerId;

    /// Marker type for [`HandlerId::of`].
    pub struct JetBrains;

    pub fn id() -> HandlerId {
        HandlerId::of::<JetBrains>()
    }
}
