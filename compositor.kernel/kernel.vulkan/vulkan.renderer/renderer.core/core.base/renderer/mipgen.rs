//! Per-surface mip-chain generation for the world anti-aliasing trilinear / anisotropic
//! modes.
//!
//! Client & iced buffers are single-mip, `SAMPLED`-only imports, so we cannot
//! `vkCmdBlitImage` FROM them (that needs TRANSFER_SRC) and cannot add transfer
//! usage to the dmabuf import without risking import failure per-modifier.
//! Instead each trilinear/aniso surface is RENDERED (sampled through the AA
//! pipeline) into an owned mip-0, then the chain is generated with blits, and
//! the AA composite samples that mipped copy with the trilinear/aniso sampler.
//!
//! **Memory optimization:** All scratch images share VkDeviceMemory slabs via
//! sub-allocation, reducing `vkAllocateMemory` calls from up to 32 to ~2-4.

use ash::vk;
use compositor_kernel_vulkan_device_factory_base::factory::VulkanDevice;

/// One owned, mipped copy of a source surface.
struct MipImage {
    image: vk::Image,
    /// Index into `MipGen::slabs` — the slab backing this image.
    slab_idx: usize,
    /// Byte offset within the slab's VkDeviceMemory.
    offset: u64,
    /// View over all mip levels — sampled by the composite.
    view: vk::ImageView,
    /// Level-0-only view — the color attachment we render the source into.
    mip0_view: vk::ImageView,
    width: u32,
    height: u32,
    mip_levels: u32,
    format: vk::Format,
    /// Memory requirements for this image (needed for sub-offset binding).
    mem_req: vk::MemoryRequirements,
}

/// A shared VkDeviceMemory slab. Multiple MipImages bind into the same
/// allocation at different offsets, reducing `vkAllocateMemory` calls.
struct MipSlab {
    memory: vk::DeviceMemory,
    size: u64,
    used: u64,
    mem_type_index: u32,
}

impl MipSlab {
    /// Try to allocate `req` bytes within this slab. Returns the offset if
    /// there's room (aligned to the requirement), or `None` if full.
    fn alloc(&mut self, req: &vk::MemoryRequirements) -> Option<u64> {
        let align = req.alignment.max(1) as u64;
        let aligned = (self.used + align - 1) & !(align - 1);
        let end = aligned + req.size;
        if end > self.size {
            return None;
        }
        self.used = end;
        Some(aligned)
    }
}

/// A pool of reusable mipped images, indexed round-robin per frame.
pub struct MipGen {
    images: Vec<MipImage>,
    slabs: Vec<MipSlab>,
    next: usize,
}

impl Default for MipGen {
    fn default() -> Self {
        Self {
            images: Vec::new(),
            slabs: Vec::new(),
            next: 0,
        }
    }
}

fn mip_count(w: u32, h: u32) -> u32 {
    32 - w.max(h).max(1).leading_zeros()
}

fn device_local(
    instance: &ash::Instance,
    phd: vk::PhysicalDevice,
    bits: u32,
) -> Option<u32> {
    let mem = unsafe { instance.get_physical_device_memory_properties(phd) };
    (0..mem.memory_type_count).find(|&i| {
        bits & (1 << i) != 0
            && mem.memory_types[i as usize]
                .property_flags
                .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
    })
}

fn subrange(base: u32, count: u32) -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: base,
        level_count: count,
        base_array_layer: 0,
        layer_count: 1,
    }
}

#[allow(clippy::too_many_arguments)]
fn barrier(
    dev: &VulkanDevice,
    cmd: vk::CommandBuffer,
    image: vk::Image,
    range: vk::ImageSubresourceRange,
    old: vk::ImageLayout,
    new: vk::ImageLayout,
    src_stage: vk::PipelineStageFlags2,
    dst_stage: vk::PipelineStageFlags2,
    src_access: vk::AccessFlags2,
    dst_access: vk::AccessFlags2,
) {
    let b = vk::ImageMemoryBarrier2::default()
        .src_stage_mask(src_stage)
        .dst_stage_mask(dst_stage)
        .src_access_mask(src_access)
        .dst_access_mask(dst_access)
        .old_layout(old)
        .new_layout(new)
        .image(image)
        .subresource_range(range);
    unsafe {
        dev.device.cmd_pipeline_barrier2(
            cmd,
            &vk::DependencyInfo::default().image_memory_barriers(std::slice::from_ref(&b)),
        );
    }
}

impl MipGen {
    /// Cap on per-frame mipped surfaces — bounds mip VRAM. Surfaces beyond this
    /// fall back to the plain composite path for the frame.
    const MAX_SURFACES: usize = 32;
    /// Initial slab size: 64 MiB. Covers ~16 surfaces at 1920×1080×4B.
    const SLAB_SIZE: u64 = 64 * 1024 * 1024;

    pub fn begin_frame(&mut self) {
        self.next = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.images.is_empty()
    }

    pub fn acquire(
        &mut self,
        dev: &VulkanDevice,
        phd: vk::PhysicalDevice,
        format: vk::Format,
        w: u32,
        h: u32,
    ) -> Result<Option<usize>, vk::Result> {
        if self.next >= Self::MAX_SURFACES {
            return Ok(None);
        }
        let (w, h) = (w.max(1), h.max(1));
        let idx = self.next;
        self.next += 1;
        let stale = match self.images.get(idx) {
            Some(m) => m.width != w || m.height != h || m.format != format,
            None => true,
        };
        if stale {
            if idx < self.images.len() {
                // Old image at this slot — destroy its views (memory stays in slab).
                let old = &self.images[idx];
                unsafe {
                    dev.device.destroy_image_view(old.view, None);
                    dev.device.destroy_image_view(old.mip0_view, None);
                    dev.device.destroy_image(old.image, None);
                }
            }
            let fresh = self.create_image(dev, phd, format, w, h)?;
            if idx < self.images.len() {
                self.images[idx] = fresh;
            } else {
                self.images.push(fresh);
            }
        }
        Ok(Some(idx))
    }

    pub fn view_of(&self, idx: usize) -> vk::ImageView {
        self.images[idx].view
    }

    /// Create a new VkImage and bind it into an existing or new slab.
    fn create_image(
        &mut self,
        dev: &VulkanDevice,
        phd: vk::PhysicalDevice,
        format: vk::Format,
        w: u32,
        h: u32,
    ) -> Result<MipImage, vk::Result> {
        let device = &dev.device;
        let levels = mip_count(w, h);
        let image = unsafe {
            device.create_image(
                &vk::ImageCreateInfo::default()
                    .image_type(vk::ImageType::TYPE_2D)
                    .format(format)
                    .extent(vk::Extent3D { width: w, height: h, depth: 1 })
                    .mip_levels(levels)
                    .array_layers(1)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .tiling(vk::ImageTiling::OPTIMAL)
                    .usage(
                        vk::ImageUsageFlags::COLOR_ATTACHMENT
                            | vk::ImageUsageFlags::SAMPLED
                            | vk::ImageUsageFlags::TRANSFER_SRC
                            | vk::ImageUsageFlags::TRANSFER_DST,
                    )
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .initial_layout(vk::ImageLayout::UNDEFINED),
                None,
            )?
        };
        let mem_req = unsafe { device.get_image_memory_requirements(image) };
        let mem_type = device_local(&dev.instance, phd, mem_req.memory_type_bits)
            .ok_or(vk::Result::ERROR_OUT_OF_DEVICE_MEMORY)?;

        // Try to find an existing slab with matching memory type and room.
        let mut slab_idx = None;
        for (i, slab) in self.slabs.iter_mut().enumerate() {
            if slab.mem_type_index == mem_type {
                if let Some(offset) = slab.alloc(&mem_req) {
                    slab_idx = Some((i, offset));
                    break;
                }
            }
        }

        // No room — allocate a new slab.
        let (slab_idx, offset) = match slab_idx {
            Some(v) => v,
            None => {
                let slab_size = Self::SLAB_SIZE.max(mem_req.size * 4);
                let memory = dev.allocate_memory(
                    &vk::MemoryAllocateInfo::default()
                        .allocation_size(slab_size)
                        .memory_type_index(mem_type),
                    "mipgen slab",
                )?;
                let idx = self.slabs.len();
                self.slabs.push(MipSlab {
                    memory,
                    size: slab_size,
                    used: mem_req.size,
                    mem_type_index: mem_type,
                });
                (idx, 0u64)
            }
        };

        unsafe {
            device.bind_image_memory(image, self.slabs[slab_idx].memory, offset)?;
        }

        let mk_view = |base, count| unsafe {
            device.create_image_view(
                &vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format)
                    .subresource_range(subrange(base, count)),
                None,
            )
        };
        let view = mk_view(0, levels)?;
        let mip0_view = mk_view(0, 1)?;

        Ok(MipImage {
            image,
            slab_idx,
            offset,
            view,
            mip0_view,
            width: w,
            height: h,
            mip_levels: levels,
            format,
            mem_req,
        })
    }

    pub fn record(
        &self,
        dev: &VulkanDevice,
        cmd: vk::CommandBuffer,
        aa: &compositor_kernel_vulkan_pipeline_composite_base::composite::AaComposite,
        fill_set: vk::DescriptorSet,
        idx: usize,
    ) {
        use compositor_kernel_vulkan_pipeline_composite_base::composite::AaPush;
        let m = &self.images[idx];
        let device = &dev.device;

        barrier(
            dev, cmd, m.image, subrange(0, 1),
            vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::PipelineStageFlags2::TOP_OF_PIPE, vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            vk::AccessFlags2::empty(), vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
        );
        let attach = vk::RenderingAttachmentInfo::default()
            .image_view(m.mip0_view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue {
                color: vk::ClearColorValue { float32: [0.0; 4] },
            });
        let area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D { width: m.width, height: m.height },
        };
        let rendering = vk::RenderingInfo::default()
            .render_area(area)
            .layer_count(1)
            .color_attachments(std::slice::from_ref(&attach));
        unsafe {
            device.cmd_begin_rendering(cmd, &rendering);
            device.cmd_set_viewport(cmd, 0, &[vk::Viewport {
                x: 0.0, y: 0.0, width: m.width as f32, height: m.height as f32,
                min_depth: 0.0, max_depth: 1.0,
            }]);
            device.cmd_set_scissor(cmd, 0, &[area]);
        }
        aa.draw(dev, cmd, fill_set, AaPush {
            dst: [-1.0, -1.0, 2.0, 2.0],
            src: [0.0, 0.0, 1.0, 1.0],
            color: [1.0, 1.0, 1.0, 1.0],
            params: [1.0, 1.0, 0.0, 0.0],
            params2: [0.0, 0.0, 0.0, 0.0],
        });
        unsafe { device.cmd_end_rendering(cmd) };

        barrier(
            dev, cmd, m.image, subrange(0, 1),
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT, vk::PipelineStageFlags2::ALL_TRANSFER,
            vk::AccessFlags2::COLOR_ATTACHMENT_WRITE, vk::AccessFlags2::TRANSFER_READ,
        );
        if m.mip_levels > 1 {
            barrier(
                dev, cmd, m.image, subrange(1, m.mip_levels - 1),
                vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::PipelineStageFlags2::TOP_OF_PIPE, vk::PipelineStageFlags2::ALL_TRANSFER,
                vk::AccessFlags2::empty(), vk::AccessFlags2::TRANSFER_WRITE,
            );
        }

        let (mut sw, mut sh) = (m.width as i32, m.height as i32);
        for level in 1..m.mip_levels {
            let (dw, dh) = ((sw / 2).max(1), (sh / 2).max(1));
            let layers = |mip| vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: mip,
                base_array_layer: 0,
                layer_count: 1,
            };
            let region = vk::ImageBlit {
                src_subresource: layers(level - 1),
                src_offsets: [
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D { x: sw, y: sh, z: 1 },
                ],
                dst_subresource: layers(level),
                dst_offsets: [
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D { x: dw, y: dh, z: 1 },
                ],
            };
            unsafe {
                device.cmd_blit_image(
                    cmd,
                    m.image, vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    m.image, vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    std::slice::from_ref(&region),
                    vk::Filter::LINEAR,
                );
            }
            barrier(
                dev, cmd, m.image, subrange(level, 1),
                vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                vk::PipelineStageFlags2::ALL_TRANSFER, vk::PipelineStageFlags2::ALL_TRANSFER,
                vk::AccessFlags2::TRANSFER_WRITE, vk::AccessFlags2::TRANSFER_READ,
            );
            sw = dw;
            sh = dh;
        }

        barrier(
            dev, cmd, m.image, subrange(0, m.mip_levels),
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::PipelineStageFlags2::ALL_TRANSFER, vk::PipelineStageFlags2::FRAGMENT_SHADER,
            vk::AccessFlags2::TRANSFER_READ, vk::AccessFlags2::SHADER_SAMPLED_READ,
        );
    }

    pub fn destroy(&mut self, dev: &VulkanDevice) {
        // Destroy images first (they reference the slabs).
        for m in self.images.drain(..) {
            unsafe {
                dev.device.destroy_image_view(m.view, None);
                dev.device.destroy_image_view(m.mip0_view, None);
                dev.device.destroy_image(m.image, None);
            }
        }
        // Then free the slabs.
        for slab in self.slabs.drain(..) {
            dev.free_memory(slab.memory);
        }
    }
}
