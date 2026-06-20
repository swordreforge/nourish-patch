//! Embeds `morph.wgsl` and registers it under
//! `embedded://compositor_background_three_lock_shader/morph.wgsl`.

use bevy::asset::embedded_asset;

pub struct MorphScenePlugin;

impl bevy::prelude::Plugin for MorphScenePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        embedded_asset!(app, "lock.shader", "morph.wgsl");
    }
}
