use compositor_support_system_trait_system_base::base::System;
use compositor_support_system_world_host_base::base::World;
use std::collections::HashMap;

/// A deferred system constructor, registered by expansions/extensions at
/// injection time (`inject(&mut SystemRegistry)` from the loader).
pub type SystemFactory = Box<dyn Fn() -> Box<dyn System>>;

/// Holds system factories grouped into named world templates, so worlds can be
/// instantiated (per output, for lock, for the selection screen) without the
/// driving layer knowing any concrete system.
#[derive(Default)]
pub struct SystemRegistry {
    templates: HashMap<&'static str, Vec<SystemFactory>>,
}

impl SystemRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a system factory to a world template (created on first use).
    pub fn add(&mut self, template: &'static str, factory: SystemFactory) {
        self.templates.entry(template).or_default().push(factory);
    }

    pub fn templates(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.templates.keys().copied()
    }

    /// Instantiate a world from a template. Panics on an unknown template —
    /// that's an injection wiring bug.
    pub fn instantiate(
        &self,
        id: uuid::Uuid,
        template: &'static str,
        world_name: &'static str,
        kernel: &compositor_support_system_storage_slot_base::base::Storage,
    ) -> World {
        let factories = self
            .templates
            .get(template)
            .unwrap_or_else(|| panic!("unknown world template '{template}'"));
        World::build(id, world_name, factories.iter().map(|f| f()).collect(), kernel)
    }
}
