use compositor_support_system_buffer_token_base::y5_buffer;
use compositor_support_system_channel_token_base::y5_channel;
use compositor_support_system_trait_system_base::base::{BufferCx, System, SystemCx, WorldBuilder};
use compositor_y5_select_interface_base::select::{SelectionCmd, SELECT_REQUEST};
use compositor_y5_select_state_base::select::CanvasSelect;
use std::any::Any;

/// Fixed id of the world-selection screen (MAIN=0, LOCK=1; see
/// compositor_y5_lock_system_base). Dormant until the picker UI lands.
pub const SELECT_WORLD: usize = 2;

/// The window-selection slot — owned/mutated by this system. The token itself
/// lives in select.state (cycle-free) so the core focus accessor can resolve it;
/// re-exported here for existing readers.
pub use compositor_y5_select_state_base::select::{SELECT, SELECT_MUT};

/// Announced after the selection set changes (carries the new count). The
/// remote bridge can listen; the system itself also broadcasts the RPC Notify.
#[derive(Clone, Copy, Debug)]
pub struct SelectionChanged {
    pub size: i32,
}
y5_channel!(pub SELECTION_CHANGED, SELECTION_CHANGED_TX: SelectionChanged);

y5_buffer!(SELECT_BUF: SelectionCmd);

/// Owns the window-selection slot. The select triggers (canvas/window input,
/// window-destroy, incoming RPC) announce `SELECT_REQUEST`; this system applies
/// it through its buffer, then broadcasts the selection-size Notify over RPC and
/// announces `SELECTION_CHANGED`.
#[derive(Default)]
pub struct SelectSystem;

impl System for SelectSystem {
    fn name(&self) -> &'static str {
        "select"
    }

    fn register(&mut self, builder: &mut WorldBuilder) {
        builder.storage.insert(&SELECT, CanvasSelect::new());
        builder.receive(&SELECT_REQUEST, Self::on_request);
    }

    fn buffer(&mut self, cx: &mut BufferCx, message: Box<dyn Any>) {
        let next = match *message.downcast::<SelectionCmd>().expect("select buffer type") {
            SelectionCmd::Set(selection) => selection,
            SelectionCmd::Clear => cx.storage.get(&SELECT).clear(),
            SelectionCmd::Remove(uuid) => {
                let (next, changed) = cx.storage.get(&SELECT).erase_uuid(uuid);
                // A stale uuid that selected nothing: no change, no broadcast.
                if !changed {
                    return;
                }
                next
            }
        };
        *cx.storage.get_mut(&SELECT_MUT) = next;

        let size = cx.storage.get(&SELECT).Selection.len() as i32;
        cx.channels.send(&SELECTION_CHANGED_TX, SelectionChanged { size });
    }
}

impl SelectSystem {
    /// Channel listener: turn an announced selection request into a self-buffer
    /// write (the only path that may mutate the slot).
    fn on_request(&mut self, cx: &mut SystemCx, cmd: &SelectionCmd) {
        cx.write(&SELECT_BUF, cmd.clone());
    }
}
