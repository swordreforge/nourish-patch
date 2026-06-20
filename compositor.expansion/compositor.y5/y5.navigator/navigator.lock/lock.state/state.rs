use std::time::Instant;

#[derive(Clone)]
pub struct NavigatorLock {
    pub set_transform: compositor_y5_camera_transform_state::state::Transform,
    pub pending_travel: Option<compositor_y5_navigator_travel_state::state::Travel>,
    pub transition_start: Instant,
}
