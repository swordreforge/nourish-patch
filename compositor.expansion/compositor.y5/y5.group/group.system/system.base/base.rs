use compositor_support_system_buffer_token_base::y5_buffer;
use compositor_support_system_trait_system_base::base::{BufferCx, System, SystemCx, WorldBuilder};
use compositor_y5_group_protocol_base::protocol::{GroupCmd, GROUP_REQUEST};
use compositor_y5_group_state_base::state::GroupState;
use compositor_y5_window_interface_record::window::LoopWindow;
use std::any::Any;
use uuid::Uuid;

/// The window-grouping slot — owned/mutated by this system. The token lives in
/// group.state (cycle-free) so the core focus accessor can resolve it;
/// re-exported here for existing readers.
pub use compositor_y5_group_state_base::state::{GROUP, GROUP_MUT};

y5_buffer!(GROUP_BUF: GroupCmd);

/// Owns the window-grouping slot. Triggers announce GROUP_REQUEST; this system
/// applies the mutation through its buffer and lowers the resulting invalidation
/// to surface messages (creates/bbox/destroys) — no direct iced-registry access.
#[derive(Default)]
pub struct GroupSystem;

impl System for GroupSystem {
    fn name(&self) -> &'static str {
        "group"
    }

    fn register(&mut self, builder: &mut WorldBuilder) {
        builder.storage.insert(&GROUP, GroupState::new());
        builder.receive(&GROUP_REQUEST, Self::on_request);
    }

    fn buffer(&mut self, cx: &mut BufferCx, message: Box<dyn Any>) {
        let invalidation = match *message.downcast::<GroupCmd>().expect("group buffer type") {
            GroupCmd::SelectionSet => {
                let windows: Vec<Uuid> = cx
                    .storage
                    .get(&compositor_y5_select_system_base::base::SELECT)
                    .Selection
                    .iter()
                    .filter_map(|w| w.uuid())
                    .collect();
                if windows.is_empty() {
                    return;
                }
                // single window -> clear its group; many -> form a new group.
                let nest = if windows.len() == 1 { None } else { Some(None) };
                cx.storage.get_mut(&GROUP_MUT).set(&windows, nest)
            }
            GroupCmd::SelectionSetJoin => {
                let resolved = {
                    let sel = cx.storage.get(&compositor_y5_select_system_base::base::SELECT);
                    let group_window = &cx.storage.get(&GROUP).window;
                    compositor_y5_group_system_resolve::resolve::resolve_join(&sel.Selection, &sel.Primary, group_window)
                };
                let Some((group, add)) = resolved else {
                    return;
                };
                cx.storage.get_mut(&GROUP_MUT).set(&add, Some(Some(group)))
            }
            GroupCmd::WindowDestroy(uuid) => cx.storage.get_mut(&GROUP_MUT).set(&vec![uuid], None),
        };

        let tx = cx
            .storage
            .get(&compositor_y5_surface_system_base::base::SURFACE)
            .surface_message_buffer_channel
            .0
            .clone();
        compositor_y5_group_system_invalidate::invalidate::send_invalidation(&tx, invalidation);
    }
}

impl GroupSystem {
    /// Channel listener: turn an announced request into a self-buffer write
    /// (the only path that may mutate the slot).
    fn on_request(&mut self, cx: &mut SystemCx, cmd: &GroupCmd) {
        cx.write(&GROUP_BUF, cmd.clone());
    }
}
