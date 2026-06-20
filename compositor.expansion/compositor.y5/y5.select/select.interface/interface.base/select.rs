use compositor_orchestration_core_state_base::Loop;
use compositor_y5_select_state_base::select::CanvasSelect;
use uuid::Uuid;

// `SelectionCmd` + the `SELECT_REQUEST`/`SELECT_REQUEST_TX` channel + the
// `announce_selection` sender moved to the Loop-FREE `select.state` crate so a
// Pass-1 input system (`CanvasSystem::input`) can announce a selection request
// without depending on `orchestration_core` (a cycle via the focus accessors).
// Re-exported here so existing rim readers + `SelectSystem`'s
// `builder.receive(&SELECT_REQUEST, …)` are unchanged.
pub use compositor_y5_select_state_base::request::{
    SelectionCmd, SELECT_REQUEST, announce_selection,
};

/// Announce a selection request to the main world's SelectSystem. The mutation
/// (and its RPC broadcast) lands when the world next drains — visibility may lag
/// by a frame, which is acceptable for selection.
fn request(state: &mut Loop, cmd: SelectionCmd) {
    announce_selection(state.inner.focus_channels(), cmd);
}

pub fn select(state: &mut Loop, selection: CanvasSelect) {
    request(state, SelectionCmd::Set(selection));
}

pub fn remove(state: &mut Loop, uuid: Uuid) {
    request(state, SelectionCmd::Remove(uuid));
}

pub fn clear(state: &mut Loop) {
    request(state, SelectionCmd::Clear);
}
