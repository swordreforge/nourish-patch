use compositor_support_system_channel_router_base::base::ChannelRouter;
use compositor_support_system_channel_token_base::y5_channel;
use crate::select::CanvasSelect;
use uuid::Uuid;

/// Selection mutation intent. The triggers (canvas/window pointer input,
/// window-destroy, incoming RPC) announce this on `SELECT_REQUEST`; the
/// `SelectSystem` owns the slot and applies it through its buffer — selection
/// is no longer mutated directly on the (dissolved) canvas monolith.
///
/// This lives in the Loop-FREE state crate (not the Loop-coupled select
/// interface) so a Pass-1 input system — `CanvasSystem::input` — can announce a
/// selection request without depending on `orchestration_core` (which would
/// cycle through the focus accessors). The Loop-coupled `select.interface`
/// re-exports it for the rim's existing `&mut Loop` callers.
#[derive(Clone)]
pub enum SelectionCmd {
    /// Replace the selection set wholesale.
    Set(CanvasSelect),
    /// Drop a window (by uuid) from the selection if present.
    Remove(Uuid),
    /// Clear the selection.
    Clear,
}

// This crate is the single owner/sender of the request channel; the SelectSystem
// (a different crate) is the receiver — owner-announced, single-sender, fan-out.
y5_channel!(pub SELECT_REQUEST, SELECT_REQUEST_TX: SelectionCmd);

/// Announce a selection request to a world's SelectSystem on its channel router.
/// Loop-free senders (a Pass-1 input system via `cx.channels`) call this; the
/// rim's `&mut Loop` wrappers in `select.interface` call it on
/// `focus_channels()`. The mutation (and its RPC broadcast) lands when the world
/// next drains — visibility may lag by a frame, which is acceptable for
/// selection.
pub fn announce_selection(channels: &mut ChannelRouter, cmd: SelectionCmd) {
    channels.send(&SELECT_REQUEST_TX, cmd);
}
