//! Presentation-feedback collection, frame callbacks, and post-frame
//! housekeeping. Replaces the `refresh()` blocks duplicated in both backends.

use smithay::desktop::utils::OutputPresentationFeedback;
use smithay::desktop::{layer_map_for_output, Window};
use smithay::output::Output;
use smithay::reexports::wayland_protocols::wp::presentation_time::server::wp_presentation_feedback;
use std::time::Duration;
use compositor_orchestration_core_state_base::Loop;

/// The presentation Kind flags this compositor reports for a hardware flip.
pub fn hw_flip_kind() -> wp_presentation_feedback::Kind {
    wp_presentation_feedback::Kind::Vsync
        | wp_presentation_feedback::Kind::HwClock
        | wp_presentation_feedback::Kind::HwCompletion
}

/// Collect presentation feedback for the windows visible in the frame that is
/// about to be queued. (Ex udev `refresh()` first half.)
pub fn collect_feedback(output: &Output, visible: &[Window]) -> OutputPresentationFeedback {
    let mut feedback = OutputPresentationFeedback::new(output);
    for window in visible {
        window.take_presentation_feedback(
            &mut feedback,
            |_, _| Some(output.clone()),
            |_, _| hw_flip_kind(),
        );
    }
    feedback
}

/// Send frame callbacks to the visible windows. `throttle` follows the
/// caller's existing behavior (`Some(Duration::ZERO)` in both backends today).
pub fn send_window_frames(state: &Loop, output: &Output, visible: &[Window]) {
    let frame_time = state.inner.start_time.elapsed();
    for window in visible {
        window.send_frame(output, frame_time, Some(Duration::ZERO), |_, _| {
            Some(output.clone())
        });
    }
}

/// Send frame callbacks to every layer of every mapped output.
/// (Identical block in both backends today.)
pub fn send_layer_frames(state: &Loop) {
    let frame_time = state.inner.start_time.elapsed();
    for output in state.inner.space_state().state.outputs() {
        let layer_map = layer_map_for_output(output);
        for layer in layer_map.layers() {
            layer.send_frame(output, frame_time, None, |_surface, _states| {
                Some(output.clone())
            });
        }
    }
}

/// Post-frame housekeeping: space refresh, popup cleanup, client flush.
/// Runs every frame, damage or no damage.
pub fn housekeeping(state: &mut Loop) {
    state.inner.space_state_mut().state.refresh();
    state.state.popup.state.cleanup();
    let _ = state.inner.loader.display_handle.flush_clients();
}
