//! CPU → VkImage staging upload (create + in-place region update).

use ash::vk;
use compositor_kernel_vulkan_device_factory_base::factory::{VulkanDevice, find_memory_type_for as find_memory_type};
use compositor_kernel_vulkan_memory_slab_base::slab::{SlabAllocator, SlabHandle};
use compositor_kernel_vulkan_renderer_error_base::VulkanError;
use smithay::backend::allocator::Fourcc;
use smithay::backend::vulkan::PhysicalDevice;
use smithay::utils::{Buffer as BufferCoord, Rectangle, Size};

/// 4 bytes per pixel — the SHM/memory formats this renderer accepts
/// (A/X RGB/BGR 8888) are all 32-bit.
const BPP: usize = 4;

/// An uploaded device-local sampled image + its backing resources.
pub struct UploadedImage {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub format: vk::Format,
    pub width: u32,
    pub height: u32,
    /// Slab handle when memory is suballocated; `None` for standalone allocs.
    pub slab: Option<SlabHandle>,
}

/// A host-visible staging buffer reused across uploads. Grows on demand; never
/// shrinks. Freed via [`StagingBuffer::destroy`] in the renderer's `Drop`.
///
/// The backing memory is persistently mapped after creation — `stage()` writes
/// through the stored pointer instead of calling map/unmap on every upload.
pub struct StagingBuffer {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    /// Persistently mapped host-visible pointer. Valid from first allocation
    /// until `destroy()`. Never unmap/re-map mid-lifetime.
    ptr: *mut u8,
    capacity: u64,
}

// SAFETY: StagingBuffer is only accessed from the render thread.
unsafe impl Send for StagingBuffer {}

impl Default for StagingBuffer {
    fn default() -> Self {
        Self {
            buffer: vk::Buffer::null(),
            memory: vk::DeviceMemory::null(),
            ptr: std::ptr::null_mut(),
            capacity: 0,
        }
    }
}

impl StagingBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ensure the staging buffer holds at least `needed` bytes, then copy `data`
    /// (`data.len() <= needed`) into it. Returns the buffer handle to copy from.
    fn stage(
        &mut self,
        dev: &VulkanDevice,
        phd: &PhysicalDevice,
        data: &[u8],
    ) -> Result<vk::Buffer, VulkanError> {
        let needed = data.len() as u64;
        let device = &dev.device;
        if self.capacity < needed {
            // Grow (and drop the old allocation). Round up to reduce churn when a
            // surface's buffer size creeps.
            let new_cap = needed.next_power_of_two().max(4096);
            unsafe {
                if self.buffer != vk::Buffer::null() {
                    device.destroy_buffer(self.buffer, None);
                }
                if self.memory != vk::DeviceMemory::null() {
                    device.free_memory(self.memory, None);
                }
            }
            let buffer = unsafe {
                device.create_buffer(
                    &vk::BufferCreateInfo::default()
                        .size(new_cap)
                        .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                        .sharing_mode(vk::SharingMode::EXCLUSIVE),
                    None,
                )?
            };
            let req = unsafe { device.get_buffer_memory_requirements(buffer) };
            let idx = find_memory_type(
                dev,
                phd,
                req.memory_type_bits,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )
            .ok_or(VulkanError::Import("no host-visible memory type".into()))?;
            let memory = unsafe {
                device
                    .allocate_memory(
                        &vk::MemoryAllocateInfo::default()
                            .allocation_size(req.size.max(new_cap))
                            .memory_type_index(idx),
                        None,
                    )
                    .inspect_err(|_| device.destroy_buffer(buffer, None))?
            };
            unsafe { device.bind_buffer_memory(buffer, memory, 0)? };
            // Persistently map the entire allocation. HOST_COHERENT guarantees
            // writes are visible to the GPU without explicit flush.
            let ptr = unsafe {
                device.map_memory(memory, 0, new_cap, vk::MemoryMapFlags::empty())?
                    as *mut u8
            };
            self.buffer = buffer;
            self.memory = memory;
            self.ptr = ptr;
            self.capacity = new_cap;
        }
        // Write through the persistent mapping — no map/unmap per upload.
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), self.ptr, data.len());
        }
        Ok(self.buffer)
    }

    pub fn destroy(&mut self, dev: &VulkanDevice) {
        unsafe {
            if !self.ptr.is_null() {
                dev.device.unmap_memory(self.memory);
                self.ptr = std::ptr::null_mut();
            }
            if self.buffer != vk::Buffer::null() {
                dev.device.destroy_buffer(self.buffer, None);
            }
            if self.memory != vk::DeviceMemory::null() {
                dev.device.free_memory(self.memory, None);
            }
        }
        self.buffer = vk::Buffer::null();
        self.memory = vk::DeviceMemory::null();
        self.capacity = 0;
    }
}

const COLOR_RANGE: vk::ImageSubresourceRange = vk::ImageSubresourceRange {
    aspect_mask: vk::ImageAspectFlags::COLOR,
    base_mip_level: 0,
    level_count: 1,
    base_array_layer: 0,
    layer_count: 1,
};

/// Run a one-time-submit command buffer synchronously (alloc → record → submit
/// → device_wait_idle → free).
fn one_time<F: FnOnce(vk::CommandBuffer)>(
    dev: &VulkanDevice,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    record: F,
) -> Result<(), VulkanError> {
    let device = &dev.device;
    let info = vk::CommandBufferAllocateInfo::default()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);
    let cmd = unsafe { device.allocate_command_buffers(&info)? }[0];
    unsafe {
        device.begin_command_buffer(
            cmd,
            &vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
        )?;
        record(cmd);
        device.end_command_buffer(cmd)?;
        let cmds = [cmd];
        let submit = vk::SubmitInfo::default().command_buffers(&cmds);
        device.queue_submit(queue, &[submit], vk::Fence::null())?;
        device.device_wait_idle()?;
        device.free_command_buffers(command_pool, &cmds);
    }
    Ok(())
}

/// Allocate a device-local SAMPLED image and upload `data` (tightly packed
/// `width*height*4`, no row padding) in full. Memory is suballocated from the
/// provided slab — the caller must NOT free the returned `memory` individually.
#[allow(clippy::too_many_arguments)]
pub fn create_and_upload(
    dev: &VulkanDevice,
    phd: &PhysicalDevice,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    staging: &mut StagingBuffer,
    slab: &mut SlabAllocator,
    data: &[u8],
    format: Fourcc,
    size: Size<i32, BufferCoord>,
) -> Result<UploadedImage, VulkanError> {
    let vk_format = compositor_kernel_vulkan_format_query_base::query::vk_format(format)
        .ok_or(VulkanError::UnsupportedFormat(format))?;
    let (width, height) = (size.w.max(1) as u32, size.h.max(1) as u32);
    let expected = width as usize * height as usize * BPP;
    if data.len() < expected {
        return Err(VulkanError::Import(format!(
            "shm buffer too small: {} < {expected} ({width}x{height})",
            data.len()
        )));
    }

    let device = &dev.device;

    // Device-local sampled image (TRANSFER_DST for the upload).
    let image_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(vk_format)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED);
    let image = unsafe { device.create_image(&image_info, None)? };
    let req = unsafe { device.get_image_memory_requirements(image) };
    let handle = slab.allocate(&req, vk::MemoryPropertyFlags::DEVICE_LOCAL)
        .map_err(|_| {
            unsafe { device.destroy_image(image, None); }
            VulkanError::Import("no device-local memory type".into())
        })?;
    let memory = handle.memory;
    unsafe { device.bind_image_memory(image, memory, handle.offset)? };

    // X-formats are opaque (no real alpha) — force alpha to 1 so the window
    // doesn't blend out transparent (see the dmabuf import path).
    let opaque = matches!(
        format,
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
    let view = unsafe {
        device.create_image_view(
            &vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk_format)
                .components(components)
                .subresource_range(COLOR_RANGE),
            None,
        )?
    };

    let buffer = staging.stage(dev, phd, &data[..expected])?;

    one_time(dev, command_pool, queue, |cmd| unsafe {
        let to_dst = [vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
            .dst_stage_mask(vk::PipelineStageFlags2::COPY)
            .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .image(image)
            .subresource_range(COLOR_RANGE)];
        device.cmd_pipeline_barrier2(cmd, &vk::DependencyInfo::default().image_memory_barriers(&to_dst));

        let region = [vk::BufferImageCopy::default()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })];
        device.cmd_copy_buffer_to_image(cmd, buffer, image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &region);

        let to_read = [vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::COPY)
            .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
            .dst_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image(image)
            .subresource_range(COLOR_RANGE)];
        device.cmd_pipeline_barrier2(cmd, &vk::DependencyInfo::default().image_memory_barriers(&to_read));
    })
    .inspect_err(|_| unsafe {
        device.destroy_image_view(view, None);
        device.destroy_image(image, None);
        // Slab memory is not freed individually — reclaimed when slab drops.
    })?;

    Ok(UploadedImage {
        image,
        memory,
        view,
        format: vk_format,
        width,
        height,
        slab: Some(handle),
    })
}

/// Re-upload a sub-`region` into an EXISTING sampled image (currently in
/// `SHADER_READ_ONLY_OPTIMAL`). `data` is the full-size buffer with row length =
/// `image_width` (the `ImportMem::update_memory` contract); `region` selects the
/// rect to copy. The region's rows are packed tightly into the staging buffer.
#[allow(clippy::too_many_arguments)]
pub fn update_region(
    dev: &VulkanDevice,
    phd: &PhysicalDevice,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    staging: &mut StagingBuffer,
    image: vk::Image,
    image_size: (u32, u32),
    data: &[u8],
    region: Rectangle<i32, BufferCoord>,
) -> Result<(), VulkanError> {
    let (img_w, img_h) = image_size;
    let rx = region.loc.x.max(0) as u32;
    let ry = region.loc.y.max(0) as u32;
    let rw = (region.size.w.max(0) as u32).min(img_w.saturating_sub(rx));
    let rh = (region.size.h.max(0) as u32).min(img_h.saturating_sub(ry));
    if rw == 0 || rh == 0 {
        return Ok(());
    }
    let src_stride = img_w as usize * BPP;
    let row_bytes = rw as usize * BPP;
    // Pack the region rows tightly into a scratch buffer, then stage that.
    let mut packed = Vec::with_capacity(row_bytes * rh as usize);
    for y in 0..rh as usize {
        let start = (ry as usize + y) * src_stride + rx as usize * BPP;
        let end = start + row_bytes;
        if end > data.len() {
            return Err(VulkanError::Import("update_memory: region row out of bounds".into()));
        }
        packed.extend_from_slice(&data[start..end]);
    }
    let buffer = staging.stage(dev, phd, &packed)?;

    let device = &dev.device;
    one_time(dev, command_pool, queue, |cmd| unsafe {
        let to_dst = [vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
            .src_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
            .dst_stage_mask(vk::PipelineStageFlags2::COPY)
            .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .old_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .image(image)
            .subresource_range(COLOR_RANGE)];
        device.cmd_pipeline_barrier2(cmd, &vk::DependencyInfo::default().image_memory_barriers(&to_dst));

        let copy = [vk::BufferImageCopy::default()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_offset(vk::Offset3D {
                x: rx as i32,
                y: ry as i32,
                z: 0,
            })
            .image_extent(vk::Extent3D {
                width: rw,
                height: rh,
                depth: 1,
            })];
        device.cmd_copy_buffer_to_image(cmd, buffer, image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &copy);

        let to_read = [vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::COPY)
            .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
            .dst_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image(image)
            .subresource_range(COLOR_RANGE)];
        device.cmd_pipeline_barrier2(cmd, &vk::DependencyInfo::default().image_memory_barriers(&to_read));
    })
}
