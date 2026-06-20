use thiserror::Error;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("dmabuf allocation: {0}")]
    Alloc(#[from] compositor_support_bevy_core_runtime_base::AllocError),

    #[error("gles import: {0}")]
    GlesImport(#[from] compositor_support_bevy_core_runtime_base::GlesImportError),

    #[error("wgpu import: {0}")]
    WgpuImport(#[from] compositor_support_bevy_core_runtime_base::WgpuImportError),

    #[error("registry has been dropped")]
    RegistryDropped,

    #[error("invalid output size: {w}x{h}")]
    InvalidSize { w: i32, h: i32 },

    #[error("gles error: {0}")]
    Gles(#[from] smithay::backend::renderer::gles::GlesError),
}
