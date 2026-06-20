//! Import a dmabuf as a render/transfer target.

use ash::vk;
use compositor_kernel_vulkan_device_factory_base::factory::VulkanDevice;
use compositor_kernel_vulkan_renderer_error_base::VulkanError;
use smithay::backend::allocator::Buffer;
use smithay::backend::allocator::dmabuf::Dmabuf;
use std::os::unix::io::{AsRawFd, BorrowedFd, IntoRawFd};

/// Imported target image + its backing memory + (optional) view.
pub type ImportedTarget = (
    vk::Image,
    vk::DeviceMemory,
    Option<vk::ImageView>,
    vk::Format,
    u32,
    u32,
);

/// Import a dmabuf as a render/transfer target. `usage` selects the image usage
/// (`COLOR_ATTACHMENT` for the bind path; `TRANSFER_DST` for the capture copy).
/// A view is created only when `make_view` is set — a TRANSFER_DST-only image
/// cannot have one. Mirrors `vulkan.memory/memory.import` but for a target
/// (rendered/copied into, not sampled).
pub fn import_target(
    dev: &VulkanDevice,
    dmabuf: &Dmabuf,
    usage: vk::ImageUsageFlags,
    make_view: bool,
) -> Result<ImportedTarget, VulkanError> {
    let fourcc = dmabuf.format().code;
    let modifier = dmabuf.format().modifier;
    let format = compositor_kernel_vulkan_format_query_base::query::vk_format(fourcc)
        .ok_or(VulkanError::UnsupportedFormat(fourcc))?;
    let size = dmabuf.size();
    let (width, height) = (size.w as u32, size.h as u32);

    let offsets: Vec<u32> = dmabuf.offsets().collect();
    let strides: Vec<u32> = dmabuf.strides().collect();
    let fds: Vec<BorrowedFd<'_>> = dmabuf.handles().collect();
    if fds.is_empty() {
        return Err(VulkanError::Import("dmabuf has no planes".into()));
    }
    let plane_layouts: Vec<vk::SubresourceLayout> = offsets
        .iter()
        .zip(strides.iter())
        .map(|(o, s)| vk::SubresourceLayout {
            offset: *o as u64,
            size: 0,
            row_pitch: *s as u64,
            array_pitch: 0,
            depth_pitch: 0,
        })
        .collect();

    let mut modifier_info = vk::ImageDrmFormatModifierExplicitCreateInfoEXT::default()
        .drm_format_modifier(Into::<u64>::into(modifier))
        .plane_layouts(&plane_layouts);
    let mut external_info = vk::ExternalMemoryImageCreateInfo::default()
        .handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);
    let image_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(format)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::DRM_FORMAT_MODIFIER_EXT)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .push_next(&mut modifier_info)
        .push_next(&mut external_info);

    let device = &dev.device;
    let image = unsafe { device.create_image(&image_info, None)? };
    let owned = fds[0]
        .try_clone_to_owned()
        .map_err(|e| VulkanError::Import(format!("fd dup: {e}")))?;
    let requirements = unsafe { device.get_image_memory_requirements(image) };

    // Intersect the image's memory requirements with the dmabuf fd's actual
    // memory types (vkGetMemoryFdPropertiesKHR), exactly as memory.import does
    // for client buffers. Picking trailing_zeros() of the image bits alone can
    // select a type the fd does not back — on NVIDIA the GPU then renders into
    // inaccessible memory and the output is black.
    let fd_loader = ash::khr::external_memory_fd::Device::new(&dev.instance, device);
    let mut fd_props = vk::MemoryFdPropertiesKHR::default();
    unsafe {
        fd_loader
            .get_memory_fd_properties(
                vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT,
                owned.as_raw_fd(),
                &mut fd_props,
            )
            .inspect_err(|_| {
                device.destroy_image(image, None);
            })
            .map_err(|e| VulkanError::Import(format!("get_memory_fd_properties: {e}")))?;
    }
    let compatible = requirements.memory_type_bits & fd_props.memory_type_bits;
    if compatible == 0 {
        unsafe { device.destroy_image(image, None) };
        return Err(VulkanError::Import(
            "no memory type compatible with both the render-target image and its dmabuf fd".into(),
        ));
    }
    let mut import_info = vk::ImportMemoryFdInfoKHR::default()
        .handle_type(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT)
        .fd(owned.into_raw_fd());
    let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
    let alloc_info = vk::MemoryAllocateInfo::default()
        .allocation_size(requirements.size)
        .memory_type_index(compatible.trailing_zeros())
        .push_next(&mut import_info)
        .push_next(&mut dedicated);
    let memory = unsafe {
        device.allocate_memory(&alloc_info, None).inspect_err(|_| {
            device.destroy_image(image, None);
        })?
    };
    unsafe {
        device.bind_image_memory(image, memory, 0)?;
    }
    let view = if make_view {
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });
        Some(unsafe { device.create_image_view(&view_info, None)? })
    } else {
        None
    };
    Ok((image, memory, view, format, width, height))
}
