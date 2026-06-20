//! The renderer's error type. Smithay's `Renderer`/`Frame` require
//! `Error: std::error::Error`.

#[derive(Debug, thiserror::Error)]
pub enum VulkanError {
    #[error("vulkan call failed: {0}")]
    Vk(String),
    #[error("dmabuf import failed: {0}")]
    Import(String),
    #[error("unsupported format: {0:?}")]
    UnsupportedFormat(smithay::backend::allocator::Fourcc),
    #[error("capability not yet implemented on the vulkan path: {0}")]
    Unimplemented(&'static str),
}

impl From<ash::vk::Result> for VulkanError {
    fn from(r: ash::vk::Result) -> Self {
        VulkanError::Vk(r.to_string())
    }
}
