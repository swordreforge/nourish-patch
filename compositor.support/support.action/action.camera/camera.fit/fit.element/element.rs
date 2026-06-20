pub use compositor_support_action_camera_fit_element_flags::{CameraPlacementFlags, PlacementResult};
use smithay::utils::{Logical, Rectangle, Size};
use compositor_support_action_camera_find_base::find::Direction;
use compositor_support_action_camera_fit_element_viewport::viewport_at;
use compositor_support_action_camera_fit_element_zoom::collect_zoom_proposals;
use compositor_support_action_camera_fit_element_zsel::select_best_zoom;
use compositor_support_action_camera_fit_element_pan::collect_pan_proposals;
use compositor_support_action_camera_fit_element_psplit::compute_pan_with_implicit_split;
use compositor_support_action_camera_fit_element_psel::select_best_pan;
use compositor_support_action_camera_fit_element_pad::apply_default_padding;

/// Compute target camera state for the given bbox and configuration.
pub fn compute_placement(
    flags: CameraPlacementFlags,
    bbox: Rectangle<f64, Logical>,
    current: PlacementResult,
    screen_size: Size<f64, Logical>,
    dir: Direction,
) -> PlacementResult {
    use CameraPlacementFlags as F;

    let zoom_proposals = collect_zoom_proposals(flags, bbox, current, screen_size);
    let target_zoom = if flags.contains(F::ZOOM_DOMINANCE) {
        select_best_zoom(flags, &zoom_proposals, current, bbox, screen_size)
    } else {
        zoom_proposals.into_iter().next().map(|p| p.zoom).unwrap_or(current.zoom)
    };

    let viewport = viewport_at(target_zoom, current.position, screen_size);

    let target_position = if flags.contains(F::PAN_DOMINANCE) {
        let pan_proposals = collect_pan_proposals(flags, bbox, current, viewport, dir);
        select_best_pan(flags, &pan_proposals, current, bbox, target_zoom, screen_size)
    } else {
        compute_pan_with_implicit_split(flags, bbox, current, viewport, dir)
    };

    let target_position = if flags.contains(F::PAD_DEFAULT) {
        apply_default_padding(target_position, target_zoom, bbox, screen_size, dir)
    } else {
        target_position
    };

    PlacementResult { position: target_position, zoom: target_zoom }
}
