use compositor_support_system_channel_router_base::base::ChannelRouter;
use compositor_support_system_persist_document_entry::base::DocumentEntry;
use compositor_support_system_persist_entry_base::base::PersistEntry;
use compositor_support_system_input_event_base::base::{InputEvent, InputFlow};
use compositor_support_system_storage_slot_base::base::Storage;
use compositor_support_system_trait_system_base::base::{BufferCx, Receiver, System, SystemCx, WorldBuilder};
use compositor_support_system_world_frame_base::base::{FramePlan, FrameTick};
use compositor_support_system_world_flow_base::flow;
use std::any::Any;
use std::collections::VecDeque;

/// Systems + their storage + event queue + buffers + input order. Worlds
/// never close — swapped-away worlds are disabled and kept.
pub struct World {
    /// Stable persistence identity (path + ledger key). Static worlds: fixed UUIDs.
    pub id: uuid::Uuid,
    pub name: &'static str,
    /// Fields are pub for the world.flow driver; mutation POLICY is write tokens.
    pub storage: Storage,
    pub channels: ChannelRouter,
    pub systems: Vec<Box<dyn System>>,
    pub receivers: Vec<Receiver>,
    pub input_order: Vec<usize>,
    pub buffers: VecDeque<(usize, Box<dyn Any>)>,
}

impl World {
    /// Run every `register()`, then rehydrate, then every `start()`.
    pub fn build(id: uuid::Uuid, name: &'static str, mut systems: Vec<Box<dyn System>>, kernel: &Storage) -> Self {
        let mut builder = WorldBuilder::new();
        for (index, system) in systems.iter_mut().enumerate() {
            builder.set_current_system(index);
            system.register(&mut builder);
        }
        let WorldBuilder { storage, receivers, mut input, .. } = builder;
        let mut world = Self {
            id,
            name,
            storage,
            channels: ChannelRouter::new(),
            systems,
            receivers,
            input_order: input.order().collect(),
            buffers: VecDeque::new(),
        };
        // Rehydrate persisted state before start()/on_enable observe it (first run = no-op).
        let slots: Vec<&'static PersistEntry> =
            world.systems.iter().flat_map(|s| s.persist().iter().copied()).collect();
        let docs: Vec<&'static DocumentEntry> =
            world.systems.iter().flat_map(|s| s.documents().iter().copied()).collect();
        compositor_support_system_persist_boot_base::base::rehydrate_world(
            id, &mut world.storage, &slots, &docs,
        );
        flow::each_system(&mut world.parts(), kernel, None, None, |system, cx| system.start(cx));
        world
    }

    pub fn storage(&self) -> &Storage { &self.storage }

    /// For registration/driver-owned slots; systems mutate only via `buffer()`.
    pub fn storage_mut(&mut self) -> &mut Storage { &mut self.storage }

    pub fn channels(&mut self) -> &mut ChannelRouter { &mut self.channels }

    pub fn enable(&mut self, kernel: &Storage) {
        flow::each_system(&mut self.parts(), kernel, None, None, |s, cx| s.on_enable(cx));
    }

    pub fn disable(&mut self, kernel: &Storage) {
        flow::each_system(&mut self.parts(), kernel, None, None, |s, cx| s.on_disable(cx));
    }

    pub fn update(&mut self, kernel: &Storage, tick: &FrameTick, platform: Option<&mut dyn Any>, seat: Option<&mut dyn Any>) {
        flow::each_system(&mut self.parts(), kernel, platform, seat, |s, cx| s.update(cx, tick));
    }

    pub fn draw(&mut self, kernel: &Storage, plan: &mut FramePlan, platform: Option<&mut dyn Any>) {
        flow::each_system(&mut self.parts(), kernel, platform, None, |s, cx| s.draw(cx, plan));
    }

    pub fn dispatch(&mut self, kernel: &Storage) { flow::dispatch(&mut self.parts(), kernel); }

    pub fn input(&mut self, kernel: &Storage, event: &InputEvent, platform: Option<&mut dyn Any>, seat: Option<&mut dyn Any>) -> InputFlow {
        flow::input(&mut self.parts(), kernel, event, platform, seat)
    }

    fn parts(&mut self) -> flow::Parts<'_> {
        flow::Parts {
            id: self.id,
            name: self.name,
            storage: &mut self.storage,
            channels: &mut self.channels,
            systems: &mut self.systems,
            receivers: &mut self.receivers,
            input_order: &mut self.input_order,
            buffers: &mut self.buffers,
        }
    }
}
