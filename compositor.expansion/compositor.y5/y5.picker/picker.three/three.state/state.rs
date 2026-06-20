//! Bevy resources, components and the external command for the picker scene.

use bevy::prelude::*;

/// Scene config, inserted at build time. `thumbnails[i]` is the bridged world
/// thumbnail for cell `i` (None → an empty, transparent cell).
#[derive(Resource)]
pub struct PickerConfig {
    pub output_handle: Handle<Image>,
    pub output_aspect: f32,
    pub thumbnails: Vec<Option<Handle<Image>>>,
    /// Per-cell occupancy: the cell holds a world (regardless of thumbnail).
    pub occupied: Vec<bool>,
}

/// The sphere's current transform, driven authoritatively from the compositor
/// (orientation + momentum + zoom live there so click-picking stays aligned).
/// `orientation` is a quaternion (xyzw); the camera is static at +Z.
#[derive(Resource)]
pub struct PickerTransform {
    pub orientation: [f32; 4],
    /// Camera zoom: distance = CAMERA_DISTANCE / zoom.
    pub zoom: f32,
}

impl Default for PickerTransform {
    fn default() -> Self {
        Self { orientation: [0.0, 0.0, 0.0, 1.0], zoom: 1.0 }
    }
}

/// The focused/selected cell (moved by arrow keys). Drives the outline + plus.
#[derive(Resource, Default)]
pub struct PickerSelected(pub Option<usize>);

/// Seconds since the scene started — drives the idle camera sway.
#[derive(Resource, Default)]
pub struct PickerClock(pub f32);

/// The selection-outline mesh handle, rebuilt to the focused cell's curved
/// border when the selection changes.
#[derive(Resource)]
pub struct PickerOutlineMesh(pub Handle<Mesh>);

/// The rotating parent of all sphere geometry (cells, wireframe, outline, plus).
#[derive(Component)]
pub struct PickerRoot;

/// The idle/sway camera.
#[derive(Component)]
pub struct PickerCamera;

/// The selection outline tile (repositioned to the selected cell).
#[derive(Component)]
pub struct PickerOutline;

/// The "+" glyph parent (shown on the selected cell only).
#[derive(Component)]
pub struct PickerPlus;

/// External command dispatched from the compositor side.
#[derive(Debug)]
pub enum PickerCommand {
    /// Set or clear the focused cell.
    SetSelected(Option<usize>),
    /// Set the sphere's absolute transform (compositor is authoritative).
    SetTransform { orientation: [f32; 4], zoom: f32 },
}
