//! Timeline semaphores — created with the export-capable handle type so they
//! can bridge to DRM syncobjs (Phase 4 Step 2, the sync-first ordering).

use ash::vk;
use compositor_kernel_vulkan_device_factory_base::factory::VulkanDevice;

#[derive(Debug, thiserror::Error)]
pub enum TimelineError {
    #[error("vkCreateSemaphore failed: {0}")]
    Create(String),
}

pub fn create(device: &VulkanDevice, initial: u64) -> Result<vk::Semaphore, TimelineError> {
    let mut type_info = vk::SemaphoreTypeCreateInfo::default()
        .semaphore_type(vk::SemaphoreType::TIMELINE)
        .initial_value(initial);
    let mut export_info = vk::ExportSemaphoreCreateInfo::default()
        .handle_types(vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD);
    let info = vk::SemaphoreCreateInfo::default()
        .push_next(&mut type_info)
        .push_next(&mut export_info);
    unsafe {
        device
            .device
            .create_semaphore(&info, None)
            .map_err(|e| TimelineError::Create(format!("{e}")))
    }
}

/// Host-side signal of a timeline point.
pub fn signal(device: &VulkanDevice, semaphore: vk::Semaphore, value: u64) -> Result<(), String> {
    let info = vk::SemaphoreSignalInfo::default()
        .semaphore(semaphore)
        .value(value);
    unsafe {
        device
            .device
            .signal_semaphore(&info)
            .map_err(|e| format!("signal_semaphore failed: {e}"))
    }
}
