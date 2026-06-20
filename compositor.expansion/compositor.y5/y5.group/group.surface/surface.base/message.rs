//! Messages emitted by the group surface.
//!
//! Boundary with the compositor (mirrors the placeholder pattern):
//! - [`GroupMessage::Collapse`] / [`GroupMessage::Show`] are the *outward*
//!   signals the compositor watches to re-size the surface. On `Show` it
//!   hands back the full padded bounding box; on `Collapse` it hands back a
//!   static 500x250 chip anchored at the same top-left origin.
//! - [`GroupMessage::Renamed`] is emitted *outward* by the UI once a rename is
//!   committed, carrying the new name, so the compositor can persist it.
//! - [`GroupMessage::SetName`] is pushed *in* by the compositor to refresh the
//!   canonical name (e.g. after an external rename). Use this — not `Renamed` —
//!   when feeding a name back into the UI, or you create a rename feedback loop.
//!
//! Everything else (`StartEdit`, `AppendChar`, `Backspace`, `Submit`, `Clear`)
//! is internal editing state driven by keyboard events.
#[derive(Debug, Clone)]
pub enum GroupMessage {
    /// Fold the group down to the compact 500x250 chip.
    Collapse,
    /// Expand the group back to its full bounding-box size.
    Show,

    /// Begin editing the group name (name was clicked).
    StartEdit,
    /// Append a typed character to the in-progress name.
    AppendChar(char),
    /// Delete the last character of the in-progress name.
    Backspace,
    /// Commit the in-progress name (Enter).
    Submit,
    /// Cancel editing and discard the in-progress name (Escape).
    Clear,

    /// Emitted *outward* by the UI once a rename is committed, carrying the new
    /// name, so the compositor can persist it.
    Renamed(String),
    /// Pushed *in* by the compositor to set the canonical name externally. Use
    /// this to feed a name back into the UI (never `Renamed`, which would loop).
    SetName(String),
}
