//! Queue selection/ownership + frame submission with a timeline-semaphore
//! signal (synchronization2). Phase 4 Step 3 — real.

use ash::vk;
use compositor_kernel_vulkan_device_factory_base::factory::VulkanDevice;

pub struct RenderQueue {
    pub queue: vk::Queue,
    pub family_index: u32,
}

pub fn graphics_queue(device: &VulkanDevice) -> RenderQueue {
    let queue = unsafe { device.device.get_device_queue(device.queue_family_index, 0) };
    RenderQueue {
        queue,
        family_index: device.queue_family_index,
    }
}

/// Submit one command buffer, signalling `timeline` at `signal_value` on
/// completion — the value `vulkan.sync` bridges to the DRM syncobj world.
pub fn submit_with_timeline(
    device: &VulkanDevice,
    queue: &RenderQueue,
    cmd: vk::CommandBuffer,
    timeline: vk::Semaphore,
    signal_value: u64,
) -> Result<(), String> {
    let cmd_info = vk::CommandBufferSubmitInfo::default().command_buffer(cmd);
    let signal_info = vk::SemaphoreSubmitInfo::default()
        .semaphore(timeline)
        .value(signal_value)
        .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS);
    let submit = vk::SubmitInfo2::default()
        .command_buffer_infos(std::slice::from_ref(&cmd_info))
        .signal_semaphore_infos(std::slice::from_ref(&signal_info));
    unsafe {
        device
            .device
            .queue_submit2(queue.queue, &[submit], vk::Fence::null())
            .map_err(|e| format!("queue_submit2 failed: {e}"))
    }
}
