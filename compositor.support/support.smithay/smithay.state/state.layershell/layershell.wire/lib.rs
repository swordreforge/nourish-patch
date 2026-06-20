#[macro_use]
extern crate compositor_developer_debug_instance_record;

use smithay::desktop::{layer_map_for_output, LayerSurface as SmithayLayerSurface};
use smithay::output::Output;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::utils::Size;
use smithay::wayland::shell::wlr_layer::{Layer, LayerSurface, WlrLayerShellState};
use compositor_support_smithay_dispatch_state_base::state::{Dispatch, DispatchWire};
use compositor_support_smithay_state_layershell_types::AnchorMode::BottomCenter;
use compositor_support_smithay_state_layershell_types::OverlayPlacement;

pub fn shell_state(dispatch: &mut Dispatch) -> &mut WlrLayerShellState {
    &mut dispatch.layershell.wlr
}

pub fn new_layer_surface(
    space: &compositor_support_smithay_state_space_base::state::SpaceState,
    surface: LayerSurface,
    output: Option<WlOutput>,
    _layer: Layer,
    namespace: String,
) {
    let wl_surface = surface.wl_surface();
    let metadata: Option<bool> = None;
    info!("new layer surface namespace={namespace:?} metadata={metadata:?}");

    let target_output: Option<Output> = output
        .as_ref()
        .and_then(|wl_out| Output::from_resource(wl_out))
        .or_else(|| space.state.outputs().next().cloned());

    let Some(output) = target_output else {
        warn!("no output available for layer surface; closing, {:?}", output);
        return;
    };

    let desktop_layer = SmithayLayerSurface::new(surface.clone(), namespace);
    let mut layer_map = layer_map_for_output(&output);
    layer_map
        .map_layer(&desktop_layer)
        .unwrap_or_else(|e| abort!("failed to map layer surface: {e:?}"));

    surface.with_pending_state(|state| {
        state.size = Size::new(0, 0).into();
    });

    smithay::wayland::compositor::with_states(wl_surface, |states| {
        states.data_map.insert_if_missing(|| OverlayPlacement {
            anchor_mode: BottomCenter,
            margin: 12,
        });
    });

    surface.send_configure();
}

pub fn layer_destroyed(space: &compositor_support_smithay_state_space_base::state::SpaceState, surface: LayerSurface) {
    for output in space.state.outputs() {
        let mut layer_map = layer_map_for_output(output);
        let to_unmap = layer_map
            .layers()
            .find(|l| l.layer_surface() == &surface)
            .cloned();
        if let Some(desktop_layer) = to_unmap {
            layer_map.unmap_layer(&desktop_layer);
        }
    }
}
