use smithay::desktop::Window;
use smithay::output::Output;
use smithay::reexports::wayland_protocols::wp::presentation_time::server::wp_presentation_feedback;
use smithay::reexports::wayland_server;
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::backend::ObjectId;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor::{TraversalAction, with_states, with_surface_tree_downward};
use smithay::wayland::presentation::{PresentationFeedbackCachedState, Refresh};
use smithay::wayland::seat::WaylandFocus;
use std::time::Duration;
use smithay::backend::drm::DrmEvent;
use smithay::reexports::drm::control::crtc;

// CHECK: Appearantely already implemented in udev which is more accurate.
// CHECK: THis package then is only for debugging a different method.
pub fn pre_presentation(
    window_visible: &Vec<Window>,
) -> Vec<smithay::wayland::presentation::PresentationFeedbackCallback> {
    let mut presentation_feedbacks = Vec::new();

    for window in window_visible {
        if let Some(surface) = window.wl_surface() {
            // Traverse the surface tree to catch subsurfaces as well
            with_surface_tree_downward(
                &surface,
                (),
                |_, _, _| TraversalAction::DoChildren(()),
                |surface, _, _| {
                    let feedbacks = with_states(surface, |states| {
                        std::mem::take(
                            &mut states
                                .cached_state
                                .get::<PresentationFeedbackCachedState>()
                                .current()
                                .callbacks,
                        )
                    });
                    presentation_feedbacks.extend(feedbacks);
                },
                |_, _, _| true,
            );
        }
    }

    presentation_feedbacks
}

pub struct SubmitInfo {
    pub output: Output,
    pub time: Duration,
    pub seq: u64,
    pub refresh: Refresh,
}

pub fn submit(
    info: &SubmitInfo,
    callbacks: Vec<smithay::wayland::presentation::PresentationFeedbackCallback>,
) {
    // CHECK: Should be per surface on its output
    for feedback in callbacks {
        feedback.presented(
            &info.output,
            info.time,
            info.refresh,
            info.seq,
            wp_presentation_feedback::Kind::Vsync,
        );
    }
}
