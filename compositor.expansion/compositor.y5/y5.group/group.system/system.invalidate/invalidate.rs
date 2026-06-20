use compositor_y5_group_protocol_base::protocol::{
    GroupBufferMessage, GroupBufferMessageBBOX, GroupBufferMessageDestroy, GroupBufferMessageHandle,
};
use compositor_y5_group_state_base::state::IcedInvalidation;
use compositor_y5_group_surface_base::ui::GroupUi;
use compositor_y5_surface_protocol_base::protocol::{SurfaceMessage, SurfaceMessageType};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use uuid::Uuid;

/// Lower a group-state invalidation into surface messages on the surface channel
/// (creates, bbox refreshes, destroys). Pure: it only sends — the rim drain
/// performs the renderer/registry work. Lifted out of the rim so the GroupSystem
/// can call it from its buffer.
pub fn send_invalidation(tx: &Sender<SurfaceMessage>, iced_invalidation: HashMap<Uuid, IcedInvalidation>) {
    let mut buffer = GroupBufferMessageHandle { new_handle: vec![] };
    let mut destroy = vec![];
    for (group_uuid, ui) in iced_invalidation {
        match ui {
            IcedInvalidation::BBOX => {
                let _ = tx.send(SurfaceMessage {
                    message: SurfaceMessageType::Group(GroupBufferMessage::BBOX(GroupBufferMessageBBOX { group_id: group_uuid })),
                });
            }
            IcedInvalidation::New => {
                buffer.new_handle.push((
                    group_uuid,
                    GroupUi::new(compositor_y5_group_surface_base::mode::Mode::Show, String::from("Group")),
                ));
            }
            IcedInvalidation::Destroy(handle) => destroy.push(handle),
        }
    }
    let _ = tx.send(SurfaceMessage { message: SurfaceMessageType::Group(GroupBufferMessage::Handle(buffer)) });
    if !destroy.is_empty() {
        let _ = tx.send(SurfaceMessage {
            message: SurfaceMessageType::Group(GroupBufferMessage::Destroy(GroupBufferMessageDestroy { handles: destroy })),
        });
    }
}
