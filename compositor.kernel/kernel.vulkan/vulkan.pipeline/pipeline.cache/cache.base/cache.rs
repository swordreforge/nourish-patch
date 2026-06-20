//! Pipeline cache. Phase 4 Step 3 — real (empty initial data; persistence is
//! a recorded follow-up).

use ash::vk;
use compositor_kernel_vulkan_device_factory_base::factory::VulkanDevice;

#[derive(Debug, thiserror::Error)]
pub enum PipelineCacheError {
    #[error("vkCreatePipelineCache failed: {0}")]
    Create(String),
}

pub fn create(device: &VulkanDevice) -> Result<vk::PipelineCache, PipelineCacheError> {
    let info = vk::PipelineCacheCreateInfo::default();
    unsafe {
        device
            .device
            .create_pipeline_cache(&info, None)
            .map_err(|e| PipelineCacheError::Create(format!("{e}")))
    }
}

pub fn destroy(device: &VulkanDevice, cache: vk::PipelineCache) {
    unsafe { device.device.destroy_pipeline_cache(cache, None) };
}
