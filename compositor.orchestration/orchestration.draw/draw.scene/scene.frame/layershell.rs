//! Layer-shell contribution as renderer-agnostic draw nodes (goal B): we carry
//! each layer's `wl_surface` + physical placement; the backend builds the
//! `WaylandSurfaceRenderElement`s at lowering time. No renderer here.

use smithay::desktop::layer_map_for_output;
use smithay::utils::{Physical, Size};
use compositor_orchestration_draw_node_base::node::SurfaceNode;
use compositor_orchestration_core_state_base::Loop;
use compositor_y5_surface_interface_base::position::layer_surface_position;

pub fn layershell(state: &mut Loop, _size: Size<i32, Physical>) -> Vec<SurfaceNode> {
    let mut nodes = vec![];

    for output in state.inner.space_state().state.outputs() {
        let output_size_logical = state
            .inner.space_state()
            .state
            .output_geometry(output)
            .map(|g| g.size)
            .unwrap_or_default();

        let scale = output.current_scale().fractional_scale();
        let layer_map = layer_map_for_output(output);

        for layer in layer_map.layers().rev() {
            // layer-shell anchors are logical; convert placement to physical.
            let location = layer_surface_position(state, layer, output_size_logical);
            let location_physical = location.to_f64().to_physical(scale).to_i32_round();
            nodes.push(SurfaceNode {
                surface: layer.wl_surface().clone(),
                location: location_physical,
                scale,
                alpha: 1.0,
            });
        }
    }

    nodes
}
