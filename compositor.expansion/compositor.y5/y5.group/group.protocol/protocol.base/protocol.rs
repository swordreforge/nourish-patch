use compositor_monitor_compositor_iced_base::HandleId;
use compositor_y5_group_surface_base::ui::GroupUi;

#[derive(Clone, Debug)]
pub enum GroupBufferMessage {
    Handle(GroupBufferMessageHandle),
    BBOX(GroupBufferMessageBBOX),
    Surface(GroupBufferMessageSurface),
    /// Tear down iced group surfaces by handle. Routed as a message so the owner
    /// of the slot need not touch the surface system's registry directly — the
    /// rim drain (which holds the renderer/registry) performs the destroy.
    Destroy(GroupBufferMessageDestroy),
}

#[derive(Clone, Debug)]
pub struct GroupBufferMessageDestroy {
    pub handles: Vec<HandleId>,
}

/// Group mutation intent. The triggers (selection keybind, window-destroy,
/// select_box) announce these on `GROUP_REQUEST`; the GroupSystem applies them
/// through its buffer. Grouping is no longer mutated directly from the rim.
#[derive(Clone, Debug)]
pub enum GroupCmd {
    /// Group the current selection (single window -> ungroup).
    SelectionSet,
    /// Join the current selection into one group (primary's group, or a shared one).
    SelectionSetJoin,
    /// Drop a destroyed window from its group.
    WindowDestroy(uuid::Uuid),
}

// group.protocol is the single owner/sender of the request channel; GroupSystem
// (a different crate) receives it. Senders call `emit()` so the pub(crate) TX
// stays internal here (owner-announced, single-sender, fan-out).
compositor_support_system_channel_token_base::y5_channel!(pub GROUP_REQUEST, GROUP_REQUEST_TX: GroupCmd);

/// Announce a group request on a world's channel router (used by the rim triggers).
pub fn emit(channels: &mut compositor_support_system_channel_router_base::base::ChannelRouter, cmd: GroupCmd) {
    channels.send(&GROUP_REQUEST_TX, cmd);
}

#[derive(Clone, Debug)]
pub struct GroupBufferMessageHandle {
    pub new_handle: Vec<(uuid::Uuid, GroupUi)>,
}

#[derive(Clone, Debug)]
pub struct GroupBufferMessageBBOX {
    pub group_id: uuid::Uuid,
}

#[derive(Clone, Debug)]
pub struct GroupBufferMessageSurface {
    pub group_id: uuid::Uuid,
    pub name: Option<String>,
    pub visibility: Option<bool>,
}
