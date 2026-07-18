//! Pipeline cache with optional disk persistence. On creation, an opaque blob
//! from a previous session can be fed via `initial_data` to warm the driver's
//! internal cache. On shutdown, `get_data` extracts the current blob for the
//! next session.

use ash::vk;
use compositor_kernel_vulkan_device_factory_base::factory::VulkanDevice;

#[derive(Debug, thiserror::Error)]
pub enum PipelineCacheError {
    #[error("vkCreatePipelineCache failed: {0}")]
    Create(String),
}

/// Create a pipeline cache, optionally warming it with data from a previous
/// session. `initial_data` is the raw blob from [`get_data`]; pass `&[]` or
/// `None` for a cold start.
pub fn create(
    device: &VulkanDevice,
    initial_data: Option<&[u8]>,
) -> Result<vk::PipelineCache, PipelineCacheError> {
    let mut info = vk::PipelineCacheCreateInfo::default();
    if let Some(data) = initial_data.filter(|d| !d.is_empty()) {
        info = info.initial_data(data);
    }
    unsafe {
        device
            .device
            .create_pipeline_cache(&info, None)
            .map_err(|e| PipelineCacheError::Create(format!("{e}")))
    }
}

/// Extract the opaque pipeline cache blob. Returns `None` if the driver
/// refuses to serialize (should not happen in practice).
pub fn get_data(device: &VulkanDevice, cache: vk::PipelineCache) -> Option<Vec<u8>> {
    unsafe { device.device.get_pipeline_cache_data(cache).ok() }
}

pub fn destroy(device: &VulkanDevice, cache: vk::PipelineCache) {
    unsafe { device.device.destroy_pipeline_cache(cache, None) };
}
