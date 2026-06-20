//! Renderable/texturable format discovery on a physical device. Phase 4
//! Step 1 — real via instance-level format property queries.

use ash::vk;
use smithay::backend::allocator::Fourcc;
use smithay::backend::vulkan::PhysicalDevice;

/// Fourcc -> VkFormat for the formats this compositor offers (mirrors the
/// scanout color format policy: Argb8888/Abgr8888 + the common client set).
pub fn vk_format(fourcc: Fourcc) -> Option<vk::Format> {
    match fourcc {
        Fourcc::Argb8888 | Fourcc::Xrgb8888 => Some(vk::Format::B8G8R8A8_UNORM),
        Fourcc::Abgr8888 | Fourcc::Xbgr8888 => Some(vk::Format::R8G8B8A8_UNORM),
        Fourcc::Argb2101010 | Fourcc::Xrgb2101010 => Some(vk::Format::A2R10G10B10_UNORM_PACK32),
        Fourcc::Abgr2101010 | Fourcc::Xbgr2101010 => Some(vk::Format::A2B10G10R10_UNORM_PACK32),
        _ => None,
    }
}

/// Whether the device can render to (color-attach) this format at all.
pub fn renderable(phd: &PhysicalDevice, format: vk::Format) -> bool {
    let props = unsafe {
        phd.instance()
            .handle()
            .get_physical_device_format_properties(phd.handle(), format)
    };
    props
        .optimal_tiling_features
        .contains(vk::FormatFeatureFlags::COLOR_ATTACHMENT)
}
