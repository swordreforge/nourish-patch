//! The trait every Bevy scene in this system implements.
//!
//! Each scene defines a `Command` type and methods to build the world and
//! apply commands.
//!
//! Texture inputs are passed to the scene's constructor as plain
//! `Handle<Image>` values, the same way any other field would be. The
//! caller obtains these handles via `BevyRegistry::import_dmabuf(...)` and
//! stores them in the scene struct. To swap a texture later, dispatch a
//! command that carries a new `Handle<Image>` and mutate the relevant
//! resource/asset inside `apply_command`.

use bevy::asset::Handle;
use bevy::image::Image;
use bevy::prelude::{App, World};

/// One Bevy scene definition.
pub trait BevyScene: Send + Sync + 'static {
    /// Typed external command dispatched via `registry.dispatch_command(...)`.
    type Command: Send + Sync + std::fmt::Debug + 'static;

    /// Add Bevy systems, entities, resources.
    ///
    /// `output` is the `Handle<Image>` to assign to a `Camera`'s render
    /// target. Backed by a dmabuf the compositor samples.
    ///
    /// Any other texture inputs the scene needs should be carried on the
    /// scene struct itself, obtained from
    /// `BevyRegistry::import_dmabuf(...)` and passed into the scene's
    /// constructor.
    fn build(&self, app: &mut App, output: Handle<Image>);

    /// Apply a typed command to the world.
    fn apply_command(&self, world: &mut World, command: Self::Command);
}
