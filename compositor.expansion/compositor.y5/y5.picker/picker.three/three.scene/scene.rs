//! `PickerScene`: the world-selection sphere of cells, and its `BevyScene` impl.
//! Sibling crates hold the geometry, spawn, per-frame systems and command body;
//! this crate is the facade the compositor instantiates.

use bevy::asset::Handle;
use bevy::image::Image;
use bevy::prelude::{App, World};
use std::sync::Arc;

pub use compositor_y5_picker_three_state::PickerCommand;

pub struct PickerScene {
    pub output_aspect: f32,
    /// Per-cell world thumbnail (None → gray). Index is the cell index.
    pub thumbnails: Vec<Option<Arc<wgpu::Texture>>>,
    /// Per-cell occupancy (cell holds a world). A cell may be occupied with no
    /// thumbnail (restored from disk) — it still renders as filled.
    pub occupied: Vec<bool>,
}

impl PickerScene {
    pub fn new(
        output_size: (u32, u32),
        thumbnails: Vec<Option<Arc<wgpu::Texture>>>,
        occupied: Vec<bool>,
    ) -> Self {
        let output_aspect = output_size.0 as f32 / output_size.1.max(1) as f32;
        Self { output_aspect, thumbnails, occupied }
    }
}

impl compositor_support_bevy_core_scene_base::BevyScene for PickerScene {
    type Command = PickerCommand;

    fn build(&self, app: &mut App, output: Handle<Image>) {
        compositor_y5_picker_three_build::build::build(
            self.thumbnails.clone(),
            self.occupied.clone(),
            self.output_aspect,
            app,
            output,
        )
    }

    fn apply_command(&self, world: &mut World, command: Self::Command) {
        compositor_y5_picker_three_build::build::apply_command(world, command)
    }
}
