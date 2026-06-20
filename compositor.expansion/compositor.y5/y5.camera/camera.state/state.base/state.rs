use compositor_support_system_storage_token_base::base::{Token, TokenMut};

/// World-storage tokens for the camera slot (beside the type — no
/// system-crate dependency needed to read camera state).
pub static CAMERA: Token<Camera> = Token::new();
/// TRANSITIONAL pub: legacy input/draw paths still write directly.
pub static CAMERA_MUT: TokenMut<Camera> = TokenMut::new(&CAMERA);

use std::collections::HashMap;
use compositor_y5_camera_transform_state::state::Transform;
use compositor_y5_camera_zone_state::state::CameraZone;

#[derive(Default)]
pub struct Camera {
    pub transform: compositor_y5_camera_transform_state::state::Transform,
    pub zone: compositor_y5_camera_zone_state::state::CameraZone,
    pub position_previous: smithay::utils::Point<f64, smithay::utils::Logical>,
}

