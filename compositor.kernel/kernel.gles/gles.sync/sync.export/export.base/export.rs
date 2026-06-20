//! GLES/EGL native-fence export (render completion -> sync_file fd).
//!
//! The GLES counterpart of the Vulkan render fence: inserts an EGL native fence
//! (`EGL_ANDROID_native_fence_sync`) into the CURRENT context and exports it as
//! a `sync_file` fd — the same kind of fence the Vulkan path produces, usable
//! for KMS IN_FENCE / explicit-sync release. The caller must have made the GLES
//! context current first (the fence marks the GPU work submitted so far).
//!
//! Note: smithay's `GlesRenderer` already exposes this via `export_sync_point()`
//! (and `DrmCompositor` consumes it for scanout), so the native GLES path gets
//! fences for free; this is the standalone capability for direct use.

use smithay::backend::egl::fence::EGLFence;
use smithay::backend::egl::EGLDisplay;
use std::os::unix::io::OwnedFd;

#[derive(Debug, thiserror::Error)]
pub enum FenceExportError {
    #[error("EGL native fence create failed (no EGL_ANDROID_native_fence_sync?): {0}")]
    Create(String),
    #[error("EGL native fence export failed: {0}")]
    Export(String),
}

/// Create + export an EGL native fence for the work submitted in the current
/// context. Returns the `sync_file` fd.
pub fn export_render_fence(display: &EGLDisplay) -> Result<OwnedFd, FenceExportError> {
    let fence = EGLFence::create(display).map_err(|e| FenceExportError::Create(format!("{e}")))?;
    fence
        .export()
        .map_err(|e| FenceExportError::Export(format!("{e}")))
}
