//! Mesh / material helpers: unlit colours, line meshes, and the CURVED cell
//! patches + borders (UV-wrapped onto the sphere so cells sit exactly on it).

use bevy::asset::RenderAssetUsages;
use bevy::math::Vec3;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use compositor_y5_picker_three_constant::{CELL_COUNT, SPHERE_RADIUS};
use compositor_y5_picker_three_layout::cell_grid_point;

/// Subdivisions per cell edge (patch + wireframe smoothness).
const RES: u32 = 6;

/// A flat, blended, unlit colour material.
pub fn unlit(color: [f32; 4]) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(color[0], color[1], color[2], color[3]),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    }
}

/// A `LineList` mesh from segment endpoints. Retains `MAIN_WORLD` so CPU
/// attributes survive extraction (the outline rebuilds positions on re-focus).
pub fn line_mesh(points: Vec<[f32; 3]>) -> Mesh {
    let usage = RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD;
    let mut mesh = Mesh::new(PrimitiveTopology::LineList, usage);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, points);
    mesh
}

/// Curved, UV-wrapped patch mesh for one cell — its surface lies on the sphere.
pub fn patch_mesh(index: usize) -> Mesh {
    let n = RES;
    let mut pos = Vec::new();
    let mut nrm = Vec::new();
    let mut uv = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    for j in 0..=n {
        for i in 0..=n {
            let su = i as f32 / n as f32;
            let tv = j as f32 / n as f32;
            let p = cell_grid_point(index, su, tv);
            pos.push(p.to_array());
            nrm.push((p / SPHERE_RADIUS).to_array());
            uv.push([su, 1.0 - tv]);
        }
    }
    let stride = n + 1;
    for j in 0..n {
        for i in 0..n {
            let a = j * stride + i;
            let (b, c, d) = (a + 1, a + stride, a + stride + 1);
            idx.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, nrm);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh.insert_indices(Indices::U32(idx));
    mesh
}

/// The perimeter points of a cell's patch, in order (a closed loop).
fn border_loop(index: usize) -> Vec<Vec3> {
    let n = RES;
    let f = |i: u32| i as f32 / n as f32;
    let mut p = Vec::new();
    for i in 0..n { p.push(cell_grid_point(index, f(i), 0.0)); }
    for j in 0..n { p.push(cell_grid_point(index, 1.0, f(j))); }
    for i in 0..n { p.push(cell_grid_point(index, 1.0 - f(i), 1.0)); }
    for j in 0..n { p.push(cell_grid_point(index, 0.0, 1.0 - f(j))); }
    p
}

/// `LineList` segment endpoints for one cell's curved border (a closed loop).
pub fn cell_border_points(index: usize) -> Vec<[f32; 3]> {
    let loop_pts = border_loop(index);
    let mut seg = Vec::with_capacity(loop_pts.len() * 2);
    for i in 0..loop_pts.len() {
        seg.push(loop_pts[i].to_array());
        seg.push(loop_pts[(i + 1) % loop_pts.len()].to_array());
    }
    seg
}

/// Every cell's curved border baked into one segment list (the wireframe).
pub fn wire_points() -> Vec<[f32; 3]> {
    (0..CELL_COUNT).flat_map(cell_border_points).collect()
}
