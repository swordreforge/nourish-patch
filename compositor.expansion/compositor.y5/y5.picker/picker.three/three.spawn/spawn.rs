//! Startup system: idle camera + the wireframe sphere of CURVED cells (occupied
//! cells UV-textured patches, empty cells transparent), plus the selection
//! outline + "+".

use bevy::camera::{Camera, Camera3d, ClearColorConfig, ImageRenderTarget, RenderTarget};
use bevy::prelude::*;
use compositor_y5_picker_three_constant::{
    CAMERA_DISTANCE, CAMERA_FOV_RAD, CELL_COUNT, OCCUPIED_COLOR, OUTLINE_COLOR, PLUS_COLOR,
    PLUS_LEN, PLUS_THICK, WIRE_COLOR,
};
use compositor_y5_picker_three_mesh::{line_mesh, patch_mesh, unlit, wire_points};
use compositor_y5_picker_three_state::{
    PickerCamera, PickerConfig, PickerOutline, PickerOutlineMesh, PickerPlus, PickerRoot,
};

pub fn spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<PickerConfig>,
) {
    commands.spawn((
        Camera3d::default(),
        Camera { clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
        Projection::Perspective(PerspectiveProjection {
            fov: CAMERA_FOV_RAD,
            aspect_ratio: config.output_aspect,
            near: 0.05,
            far: 100.0,
            ..default()
        }),
        Transform::from_xyz(0.0, 0.0, CAMERA_DISTANCE).looking_at(Vec3::ZERO, Vec3::Y),
        RenderTarget::Image(ImageRenderTarget::from(config.output_handle.clone())),
        PickerCamera,
    ));

    // Pre-build assets so the spawn closure borrows nothing mutably.
    let wire_mesh = meshes.add(line_mesh(wire_points()));
    let wire_mat = materials.add(unlit(WIRE_COLOR));
    // One curved patch mesh per OCCUPIED cell: textured with the world thumbnail
    // when present, else a solid fill (e.g. a world restored from disk before it
    // is next active and re-captured) — either way the cell reads as occupied.
    let occupied: Vec<(Handle<Mesh>, Handle<StandardMaterial>)> = (0..CELL_COUNT)
        .filter_map(|i| {
            let tex = config.thumbnails.get(i).and_then(|t| t.clone());
            if tex.is_none() && !config.occupied.get(i).copied().unwrap_or(false) {
                return None; // empty cell — no patch.
            }
            let mat = match tex {
                Some(tex) => materials.add(StandardMaterial {
                    base_color_texture: Some(tex), unlit: true, cull_mode: None, ..default()
                }),
                None => materials.add(unlit(OCCUPIED_COLOR)),
            };
            Some((meshes.add(patch_mesh(i)), mat))
        })
        .collect();
    let outline_handle = meshes.add(line_mesh(Vec::new()));
    let outline_mat = materials.add(unlit(OUTLINE_COLOR));
    let bar = meshes.add(Rectangle::new(1.0, 1.0));
    let plus_mat = materials.add(unlit(PLUS_COLOR));

    let outline_mesh = outline_handle.clone();
    commands
        .spawn((PickerRoot, Transform::IDENTITY, Visibility::Visible))
        .with_children(|root| {
            root.spawn((Mesh3d(wire_mesh), MeshMaterial3d(wire_mat), Transform::IDENTITY));
            for (mesh, mat) in occupied {
                root.spawn((Mesh3d(mesh), MeshMaterial3d(mat), Transform::IDENTITY));
            }
            root.spawn((
                Mesh3d(outline_handle),
                MeshMaterial3d(outline_mat),
                Transform::IDENTITY,
                Visibility::Hidden,
                PickerOutline,
            ));
            root.spawn((PickerPlus, Transform::IDENTITY, Visibility::Hidden))
                .with_children(|plus| {
                    plus.spawn((
                        Mesh3d(bar.clone()),
                        MeshMaterial3d(plus_mat.clone()),
                        Transform::from_scale(Vec3::new(PLUS_LEN, PLUS_THICK, 1.0)),
                    ));
                    plus.spawn((
                        Mesh3d(bar.clone()),
                        MeshMaterial3d(plus_mat.clone()),
                        Transform::from_scale(Vec3::new(PLUS_THICK, PLUS_LEN, 1.0)),
                    ));
                });
        });

    commands.insert_resource(PickerOutlineMesh(outline_mesh));
}
