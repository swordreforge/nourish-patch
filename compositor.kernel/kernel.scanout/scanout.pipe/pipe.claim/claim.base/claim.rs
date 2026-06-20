//! Pipe (CRTC) claim policy: current-encoder walk, crtcs[0] fallback.
//! (Ex wire.rs `new()` step 6, the CRTC half.)

use smithay::backend::drm::DrmDevice;
use smithay::reexports::drm::control::{connector, crtc, Device, ResourceHandles};

pub fn claim(
    drm: &DrmDevice,
    connector: &connector::Info,
    res: &ResourceHandles,
) -> Option<crtc::Handle> {
    connector
        .current_encoder()
        .and_then(|enc_handle| drm.get_encoder(enc_handle).ok())
        .and_then(|encoder| encoder.crtc())
        .or_else(|| res.crtcs().first().copied())
}
