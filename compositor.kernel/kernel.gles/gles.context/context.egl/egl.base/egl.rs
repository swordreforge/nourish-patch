//! EGL context construction with the High-priority policy + capability probe.
//! (Ex wire.rs GpuManager factory closure body.)

use smithay::backend::egl::context::ContextPriority;
use smithay::backend::egl::{EGLContext, EGLDisplay};
use smithay::backend::renderer::gles::{GlesError, GlesRenderer};

/// The context priority policy for compositor render contexts.
pub fn context_priority() -> ContextPriority {
    ContextPriority::High
}

/// Build a GlesRenderer for an EGL display: high-priority context, then
/// capability-probed renderer construction.
pub fn create(display: &EGLDisplay) -> Result<GlesRenderer, GlesError> {
    // EGL context creation yields egl::Error, which does not convert into
    // GlesError; per the crash-first policy (§12.1) a failed assembly-time
    // context is not self-recovering, so panic with the cause.
    let context = EGLContext::new_with_priority(display, context_priority())
        .unwrap_or_else(|e| abort!("EGL context creation failed: {e:?}"));
    let capabilities = unsafe { GlesRenderer::supported_capabilities(&context)? };
    Ok(unsafe { GlesRenderer::with_capabilities(context, capabilities)? })
}
