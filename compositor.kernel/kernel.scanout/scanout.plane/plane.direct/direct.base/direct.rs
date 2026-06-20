//! Direct-scanout policy surface: the toggle the frame executor consults.
//! Delegates the flag construction to `plane.assign` (single source).
//! Today's policy: enabled — identical behavior to the original
//! FrameFlags::DEFAULT.

use compositor_kernel_scanout_plane_assign_base::assign::{frame_flags, PlanePolicy};
use smithay::backend::drm::compositor::FrameFlags;

/// The compositor's current direct-scanout policy.
pub fn enabled() -> bool {
    true
}

pub fn flags() -> FrameFlags {
    frame_flags(PlanePolicy {
        allow_direct_scanout: enabled(),
    })
}
