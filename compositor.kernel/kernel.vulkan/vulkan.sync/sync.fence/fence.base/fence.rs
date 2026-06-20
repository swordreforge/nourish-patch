//! Smithay `Fence` implementations backing the Vulkan async render path's
//! `SyncPoint`. `TimelineFence` is the CPU-side render-completion fence
//! (timeline semaphore value); `SyncFileFence` (sibling crate) is the
//! fd-exportable variant.

use ash::vk;
use smithay::backend::renderer::sync::{Fence, Interrupted};
use std::os::unix::io::OwnedFd;

pub use compositor_kernel_vulkan_sync_fence_syncfile::syncfile::SyncFileFence;

/// A smithay `Fence` backed by a Vulkan timeline semaphore value: `wait()` is
/// `vkWaitSemaphores`, `is_signaled()` is `vkGetSemaphoreCounterValue`. Not
/// fd-exportable, so native CPU-waits via `needs_sync()`. (On NVIDIA a host
/// wait on a deferred GPU signal can hang; the async path is opt-in.)
pub struct TimelineFence {
    device: ash::Device,
    semaphore: vk::Semaphore,
    value: u64,
}

impl std::fmt::Debug for TimelineFence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimelineFence").field("value", &self.value).finish()
    }
}

impl TimelineFence {
    pub fn new(device: ash::Device, semaphore: vk::Semaphore, value: u64) -> Self {
        Self { device, semaphore, value }
    }
}

impl Fence for TimelineFence {
    fn is_signaled(&self) -> bool {
        unsafe {
            self.device
                .get_semaphore_counter_value(self.semaphore)
                .map(|v| v >= self.value)
                .unwrap_or(true)
        }
    }

    fn wait(&self) -> Result<(), Interrupted> {
        let sems = [self.semaphore];
        let vals = [self.value];
        let info = vk::SemaphoreWaitInfo::default().semaphores(&sems).values(&vals);
        unsafe { self.device.wait_semaphores(&info, u64::MAX).map_err(|_| Interrupted) }
    }

    fn is_exportable(&self) -> bool {
        false
    }

    fn export(&self) -> Option<OwnedFd> {
        None
    }
}
