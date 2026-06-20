//! dmabuf -> VkImage (client buffer import) — the vulkan implementation of
//! the contract import capability. Phase 4 Step 3 — real for the
//! single-memory (non-disjoint) plane layout, which covers the formats this
//! compositor offers; disjoint multi-plane import is the recorded follow-up.

use ash::vk;
use compositor_kernel_vulkan_device_factory_base::factory::VulkanDevice;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::Buffer;
use smithay::backend::vulkan::PhysicalDevice;
use std::os::unix::io::{AsRawFd, BorrowedFd};

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("unsupported fourcc for the vulkan path: {0:?}")]
    UnsupportedFormat(smithay::backend::allocator::Fourcc),
    #[error("disjoint multi-plane import not populated (single-memory path only)")]
    Disjoint,
    #[error("vulkan call failed: {0}")]
    Vk(String),
}

pub struct ImportedImage {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub format: vk::Format,
    pub size: (u32, u32),
}

/// Contract validation: a full import attempt followed by teardown.
pub fn validate(device: &VulkanDevice, phd: &PhysicalDevice, dmabuf: &Dmabuf) -> bool {
    match import(device, phd, dmabuf) {
        Ok(img) => {
            destroy(device, img);
            true
        }
        Err(e) => {
            trace!("vulkan dmabuf validation failed: {e}");
            false
        }
    }
}

pub fn import(
    device: &VulkanDevice,
    _phd: &PhysicalDevice,
    dmabuf: &Dmabuf,
) -> Result<ImportedImage, ImportError> {
    let fourcc = dmabuf.format().code;
    let modifier = dmabuf.format().modifier;
    let format = compositor_kernel_vulkan_format_query_base::query::vk_format(fourcc)
        .ok_or(ImportError::UnsupportedFormat(fourcc))?;
    let size = dmabuf.size();
    let (width, height) = (size.w as u32, size.h as u32);

    // Per-plane layouts from the dmabuf description.
    let offsets: Vec<u32> = dmabuf.offsets().collect();
    let strides: Vec<u32> = dmabuf.strides().collect();
    let fds: Vec<BorrowedFd<'_>> = dmabuf.handles().collect();
    if fds.is_empty() {
        return Err(ImportError::Vk("dmabuf has no planes".into()));
    }
    // Single-memory path: all planes must reference one fd region.
    let first_fd = fds[0].as_raw_fd();
    if fds.iter().any(|fd| fd.as_raw_fd() != first_fd) && fds.len() > 1 {
        return Err(ImportError::Disjoint);
    }

    let plane_layouts: Vec<vk::SubresourceLayout> = offsets
        .iter()
        .zip(strides.iter())
        .map(|(offset, stride)| vk::SubresourceLayout {
            offset: *offset as u64,
            size: 0, // must be 0 for VK_EXT_image_drm_format_modifier
            row_pitch: *stride as u64,
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
        .usage(vk::ImageUsageFlags::SAMPLED)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .push_next(&mut modifier_info)
        .push_next(&mut external_info);

    let dev = &device.device;
    let image = unsafe {
        dev.create_image(&image_info, None)
            .map_err(|e| ImportError::Vk(format!("create_image: {e}")))?
    };

    let requirements = unsafe { dev.get_image_memory_requirements(image) };

    // dup the fd: used for both the memory-type query and the import
    // (vkAllocateMemory consumes the fd it is handed).
    let owned = fds[0].try_clone_to_owned().map_err(|e| {
        unsafe { dev.destroy_image(image, None) };
        ImportError::Vk(format!("fd dup: {e}"))
    })?;

    // The imported allocation MUST use a memory type valid for THIS dmabuf fd
    // (per VkMemoryFdPropertiesKHR), intersected with the image's requirements.
    // Selecting an arbitrary type (the lowest requirement bit) binds the imported
    // memory to an incompatible type — it may "work" without validation layers
    // but faults the GPU when the image is later sampled (the iced-import crash).
    let fd_loader = ash::khr::external_memory_fd::Device::new(&device.instance, dev);
    let mut fd_props = vk::MemoryFdPropertiesKHR::default();
    unsafe {
        fd_loader
            .get_memory_fd_properties(
                vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT,
                owned.as_raw_fd(),
                &mut fd_props,
            )
            .map_err(|e| {
                dev.destroy_image(image, None);
                ImportError::Vk(format!("get_memory_fd_properties: {e}"))
            })?;
    }
    let compatible = requirements.memory_type_bits & fd_props.memory_type_bits;
    if compatible == 0 {
        unsafe { dev.destroy_image(image, None) };
        return Err(ImportError::Vk(
            "no memory type compatible with both the image and the dmabuf fd".into(),
        ));
    }
    let memory_type_index = compatible.trailing_zeros();

    let mut import_info = vk::ImportMemoryFdInfoKHR::default()
        .handle_type(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT)
        .fd({
            use std::os::unix::io::IntoRawFd;
            owned.into_raw_fd()
        });
    let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
    let alloc_info = vk::MemoryAllocateInfo::default()
        .allocation_size(requirements.size)
        .memory_type_index(memory_type_index)
        .push_next(&mut import_info)
        .push_next(&mut dedicated);

    let memory = unsafe {
        dev.allocate_memory(&alloc_info, None).map_err(|e| {
            dev.destroy_image(image, None);
            ImportError::Vk(format!("allocate_memory: {e}"))
        })?
    };
    unsafe {
        dev.bind_image_memory(image, memory, 0).map_err(|e| {
            dev.free_memory(memory, None);
            dev.destroy_image(image, None);
            ImportError::Vk(format!("bind_image_memory: {e}"))
        })?;
    }

    // Opaque (X-prefixed) formats have no real alpha — the X byte is undefined,
    // so clients (Blender, vkcube) often leave it 0. Vulkan maps Xrgb8888 →
    // B8G8R8A8_UNORM (which HAS alpha), so sampling that 0 as alpha makes the
    // window blend out transparent. Force the alpha channel to 1 via a view
    // swizzle for X-formats; ARGB/ABGR keep real alpha.
    use smithay::backend::allocator::Fourcc;
    let opaque = matches!(
        fourcc,
        Fourcc::Xrgb8888 | Fourcc::Xbgr8888 | Fourcc::Xrgb2101010 | Fourcc::Xbgr2101010
    );
    let components = vk::ComponentMapping {
        r: vk::ComponentSwizzle::IDENTITY,
        g: vk::ComponentSwizzle::IDENTITY,
        b: vk::ComponentSwizzle::IDENTITY,
        a: if opaque {
            vk::ComponentSwizzle::ONE
        } else {
            vk::ComponentSwizzle::IDENTITY
        },
    };
    let view_info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .components(components)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });
    let view = unsafe {
        dev.create_image_view(&view_info, None).map_err(|e| {
            dev.free_memory(memory, None);
            dev.destroy_image(image, None);
            ImportError::Vk(format!("create_image_view: {e}"))
        })?
    };

    Ok(ImportedImage {
        image,
        memory,
        view,
        format,
        size: (width, height),
    })
}

pub fn destroy(device: &VulkanDevice, img: ImportedImage) {
    unsafe {
        device.device.destroy_image_view(img.view, None);
        device.device.destroy_image(img.image, None);
        device.device.free_memory(img.memory, None);
    }
}
