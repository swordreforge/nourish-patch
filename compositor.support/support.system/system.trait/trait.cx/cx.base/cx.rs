use compositor_support_system_buffer_token_base::base::Buffer;
use compositor_support_system_channel_router_base::base::ChannelRouter;
use compositor_support_system_channel_token_base::base::ChannelTx;
use compositor_support_system_storage_slot_base::base::Storage;
use std::any::Any;
use std::collections::VecDeque;

/// Lifecycle context: storage is READ-ONLY here. Mutation intent goes through
/// `write()` (the system's own buffer) and is applied in `System::buffer()`.
/// `platform` is a TRANSITIONAL escape hatch (phase 4 removes it).
pub struct SystemCx<'a> {
    pub storage: &'a Storage,
    /// KernelData: smithay wiring handles (display/loop/seat) behind the same
    /// token pattern — read-only for systems, populated by the driving layer.
    pub kernel: &'a Storage,
    pub channels: &'a mut ChannelRouter,
    pub platform: Option<&'a mut dyn Any>,
    /// Seat capability: the wayland `Dispatch` (the seat/protocol state), lent by
    /// the rim DISJOINTLY from the world for synchronous seat ops (now possible
    /// because `D = Dispatch` is a field, not the whole `Loop` — see
    /// document/SMITHAY_DECOUPLING.md "P3"). `dyn Any` keeps this crate free of a
    /// smithay dep; the consuming input system downcasts to `Dispatch`. Present on
    /// the input-bus path; `None` on update/dispatch/buffer.
    pub seat: Option<&'a mut dyn Any>,
    pub buffers: &'a mut VecDeque<(usize, Box<dyn Any>)>,
    pub system: usize,
}

impl<'a> SystemCx<'a> {
    /// Built by the world per lifecycle call; `system` is the running system's
    /// index — buffer writes are self-addressed through it.
    pub fn new(
        storage: &'a Storage,
        kernel: &'a Storage,
        channels: &'a mut ChannelRouter,
        platform: Option<&'a mut dyn Any>,
        seat: Option<&'a mut dyn Any>,
        buffers: &'a mut VecDeque<(usize, Box<dyn Any>)>,
        system: usize,
    ) -> Self {
        Self { storage, kernel, channels, platform, seat, buffers, system }
    }

    /// Announce an event on a channel this crate owns the sender for.
    pub fn send<M: 'static>(&mut self, tx: &'static ChannelTx<M>, message: M) {
        self.channels.send(tx, message);
    }

    /// Queue a self-addressed mutation message; delivered to THIS system's
    /// `buffer()` when the world flushes (same phase).
    pub fn write<M: 'static>(&mut self, _buffer: &'static Buffer<M>, message: M) {
        self.buffers.push_back((self.system, Box::new(message)));
    }
}

/// The ONLY context holding mutable storage — handed exclusively to
/// `System::buffer()`. Write tokens still gate which slots a crate may touch.
pub struct BufferCx<'a> {
    pub storage: &'a mut Storage,
    /// KernelData (see SystemCx::kernel) — read-only here too.
    pub kernel: &'a Storage,
    pub channels: &'a mut ChannelRouter,
    /// The building/running world's id — for flagging a persisted mutation via
    /// `compositor_support_system_persist_mark_base::base::mark_world(cx.world, …)`.
    pub world: uuid::Uuid,
}

impl<'a> BufferCx<'a> {
    /// Apply an in-memory storage mutation and flag it for persistence. The closure
    /// mutates `storage` immediately; `immediate=true` commits at THIS buffer
    /// boundary (`flow::flush`), `false` debounces it (batched up to 1s) so the disk
    /// write lands later — off the mutation frame — without spamming disk on a slot
    /// (e.g. a placeholder transform) that mutates every frame.
    pub fn transact(&mut self, immediate: bool, f: impl FnOnce(&mut Storage)) {
        f(self.storage);
        compositor_support_system_persist_mark_base::base::mark_world(self.world, immediate);
    }

    /// Flag the world's persisted state dirty without wrapping the mutation (it
    /// already touched `self.storage`). Same debounce semantics as `transact`.
    pub fn mark(&mut self, immediate: bool) {
        compositor_support_system_persist_mark_base::base::mark_world(self.world, immediate);
    }
}

