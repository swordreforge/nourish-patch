//! Command pool / per-frame command buffer management. Phase 4 Step 3
//! skeleton with the real creation shape.

use ash::vk;
use compositor_kernel_vulkan_device_factory_base::factory::VulkanDevice;

#[derive(Debug, thiserror::Error)]
pub enum CommandPoolError {
    #[error("vkCreateCommandPool failed: {0}")]
    Create(String),
}

pub fn create(device: &VulkanDevice) -> Result<vk::CommandPool, CommandPoolError> {
    let info = vk::CommandPoolCreateInfo::default()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(device.queue_family_index);
    unsafe {
        device
            .device
            .create_command_pool(&info, None)
            .map_err(|e| CommandPoolError::Create(format!("{e}")))
    }
}
