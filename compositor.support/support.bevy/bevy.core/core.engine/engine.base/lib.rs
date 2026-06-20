//! # compositor_support_bevy_core_engine_base
//!
//! Facade: per-instance Bevy `App`, scene trait and typed command dispatch
//! now live in flat sibling crates; every public path this crate
//! historically exposed keeps resolving through the re-exports below.

pub mod error;

pub mod bridges {
    pub use compositor_support_bevy_core_bridge_base::{
        BridgeDirection, BridgeEntry, BridgeRegistry,
    };
    pub use compositor_support_bevy_core_install_base::BridgeRegistryPlugin;
    pub use compositor_support_bevy_core_placeholder_base::{
        create_input_placeholder, create_output_placeholder,
    };
}
pub mod runtime {
    pub use compositor_support_bevy_core_host_base::{BevyRuntime, CommandHandler};
}
pub mod scene {
    pub use compositor_support_bevy_core_scene_base::BevyScene;
}
pub mod shared {
    pub use compositor_support_bevy_core_shared_base::SharedContext;
}

pub use bridges::{
    BridgeDirection, BridgeEntry, BridgeRegistry, BridgeRegistryPlugin, create_input_placeholder,
    create_output_placeholder,
};
pub use error::EngineInitError;
pub use runtime::{BevyRuntime, CommandHandler};
pub use scene::BevyScene;
pub use shared::SharedContext;

pub use bevy::prelude::{App, Handle, Image, World};
