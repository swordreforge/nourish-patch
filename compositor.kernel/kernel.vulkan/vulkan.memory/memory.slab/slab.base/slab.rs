//! Generic bump-allocated VkDeviceMemory slab for DEVICE_LOCAL sub-allocation.
//!
//! Reduces `vkAllocateMemory` calls by co-allocating multiple images/buffers
//! into shared VkDeviceMemory blocks. Bump allocation is O(1); slab space is
//! not reclaimed when individual resources are freed — acceptable for
//! session-scoped compositor resources that rarely resize.

use ash::vk;
use compositor_kernel_vulkan_device_factory_base::factory::VulkanDevice;

/// Handle returned by [`SlabAllocator::allocate`]. The caller binds the
/// returned offset to its own VkImage/VkBuffer. The handle is needed to
/// prevent the memory from being freed while the resource is alive.
pub struct SlabHandle {
    pub memory: vk::DeviceMemory,
    pub offset: u64,
    pub size: u64,
}

struct Slab {
    memory: vk::DeviceMemory,
    size: u64,
    used: u64,
    mem_type_index: u32,
}

impl Slab {
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

pub struct SlabAllocator {
    dev: VulkanDevice,
    phd: vk::PhysicalDevice,
    slabs: Vec<Slab>,
}

impl SlabAllocator {
    pub fn new(dev: VulkanDevice, phd: vk::PhysicalDevice) -> Self {
        Self {
            dev,
            phd,
            slabs: Vec::new(),
        }
    }

    /// Allocate device memory from a slab. The caller must bind the returned
    /// offset to their VkImage/VkBuffer via `bind_image_memory` /
    /// `bind_buffer_memory`.
    pub fn allocate(
        &mut self,
        req: &vk::MemoryRequirements,
        props: vk::MemoryPropertyFlags,
    ) -> Result<SlabHandle, vk::Result> {
        let mem_type =
            find_memory_type(&self.dev.instance, self.phd, req.memory_type_bits, props)
                .ok_or(vk::Result::ERROR_OUT_OF_DEVICE_MEMORY)?;

        // Try existing slabs with matching memory type.
        for slab in &mut self.slabs {
            if slab.mem_type_index == mem_type {
                if let Some(offset) = slab.alloc(req) {
                    return Ok(SlabHandle {
                        memory: slab.memory,
                        offset,
                        size: req.size,
                    });
                }
            }
        }

        // Allocate a new slab — at least 4× the request to amortize.
        let slab_size = (req.size * 4).max(256 * 1024);
        let memory = self.dev.allocate_memory(
            &vk::MemoryAllocateInfo::default()
                .allocation_size(slab_size)
                .memory_type_index(mem_type),
            "slab",
        )?;
        let mut slab = Slab {
            memory,
            size: slab_size,
            used: req.size,
            mem_type_index: mem_type,
        };
        let offset = slab.alloc(req).unwrap();
        self.slabs.push(slab);

        Ok(SlabHandle {
            memory,
            offset,
            size: req.size,
        })
    }

    pub fn destroy(&mut self) {
        for slab in self.slabs.drain(..) {
            self.dev.free_memory(slab.memory);
        }
    }
}

fn find_memory_type(
    instance: &ash::Instance,
    phd: vk::PhysicalDevice,
    type_bits: u32,
    props: vk::MemoryPropertyFlags,
) -> Option<u32> {
    let mem = unsafe { instance.get_physical_device_memory_properties(phd) };
    (0..mem.memory_type_count).find(|&i| {
        (type_bits & (1 << i)) != 0
            && mem.memory_types[i as usize]
                .property_flags
                .contains(props)
    })
}
