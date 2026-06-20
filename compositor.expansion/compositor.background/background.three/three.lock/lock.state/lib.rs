//! Shared state of the morph scene: command + phase enums, config and
//! animation resources.

use bevy::asset::Handle;
use bevy::image::Image;
use bevy::prelude::{Component, Resource};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum MorphCommand {
    Lock,
    Unlock,
    SetPhase(MorphPhase),
    SetSnapshot(Arc<wgpu::Texture>),
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MorphPhase {
    #[default]
    Idle,
    PreMorphDelay,
    Morphing,
    SphereFull,
    ShrinkingToHero,
    Hero,
    GrowingFromHero,
    SphereFullReverse,
    Unmorphing,
}

pub const SNAPSHOT_LABEL: &str = "morph_snapshot";

#[derive(Resource)]
pub struct MorphConfig {
    pub snapshot_handle: Handle<Image>,
    pub output_handle: Handle<Image>,
    pub output_aspect: f32,
}

/// Animation state.
///
/// `flatness_*` semantics: 0.0 = fully sphere (stored mesh shape, no
/// modification), 1.0 = fully flat plane. Default initial state at
/// `MorphAnim::default()` is sphere (everything 0). The very first frame
/// before any Lock command shows the sphere.
#[derive(Resource, Default, Debug, Clone)]
pub struct MorphAnim {
    pub phase: MorphPhase,
    pub phase_started_at: f64,
    /// Current animation progress in [0, 1]. Driven by spring during
    /// active morph/unmorph phases.
    pub t: f32,
    /// 1.0 = morph cycle (Lock direction), 0.0 = unmorph cycle (Unlock direction).
    /// Set when entering PreMorphDelay or Unmorphing.
    pub going_to_sphere: f32,
    pub hero: f32,
}

#[derive(Component)]
pub struct MorphPlane;
