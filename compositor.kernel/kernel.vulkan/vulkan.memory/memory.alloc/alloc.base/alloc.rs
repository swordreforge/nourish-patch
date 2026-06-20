//! Scanout-capable image allocation — delegation to smithay's
//! VulkanAllocator (Phase 4 Step 1; the GBM-replacement on the vulkan path).

use smithay::backend::allocator::vulkan::{ImageUsageFlags, VulkanAllocator};
use smithay::backend::vulkan::PhysicalDevice;

/// The default usage for scanout-capable render targets.
pub fn default_usage() -> ImageUsageFlags {
    ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::SAMPLED
}

pub fn allocator(phd: &PhysicalDevice) -> Result<VulkanAllocator, String> {
    VulkanAllocator::new(phd, default_usage())
        .map_err(|e| format!("vulkan allocator creation failed: {e}"))
}
