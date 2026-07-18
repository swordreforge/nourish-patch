//! Logical VkDevice creation with the compositor's required extensions and
//! Vulkan 1.3 core features. Feature policy: timeline semaphores (1.2 core),
//! dynamic rendering + synchronization2 (1.3 core); promoted
//! VK_KHR_timeline_semaphore is NOT requested.

use ash::vk;
use smithay::backend::vulkan::PhysicalDevice;
use std::ffi::{CStr, c_char};

/// MASTER GATE for Intel-CCS / multi-plane dmabuf support: disjoint multi-plane
/// import + the VK_QUEUE_FAMILY_FOREIGN_EXT acquire that samples the compressed
/// planes. Read across the vulkan import path. Off = single-plane-only import.
pub const MULTIPLANE_SUPPORT: bool = true;

/// Device extensions the render path requires.
pub fn required_extensions() -> Vec<&'static CStr> {
    let mut ext = vec![
        ash::khr::external_memory_fd::NAME,
        ash::ext::external_memory_dma_buf::NAME,
        ash::ext::image_drm_format_modifier::NAME,
        ash::khr::external_semaphore_fd::NAME,
    ];
    if MULTIPLANE_SUPPORT {
        ext.push(ash::ext::queue_family_foreign::NAME);
    }
    ext
}

pub struct VulkanDevice {
    pub device: ash::Device,
    pub queue_family_index: u32,
    /// The owning `ash::Instance`, retained so device-level extension loaders
    /// (`ash::khr/ext::*::Device::new`) can be constructed — they need the
    /// instance to resolve `vkGetDeviceProcAddr`.
    pub instance: ash::Instance,
    /// Driver limit on concurrent `vkAllocateMemory` calls. Tracked to
    /// prevent exceeding the limit (Intel: 4096, NVIDIA: 4096, AMD: 4294967295).
    pub max_allocations: u32,
    /// Current number of live `VkDeviceMemory` allocations.
    pub allocation_count: std::cell::Cell<u32>,
}

impl VulkanDevice {
    /// Check whether a new allocation would exceed the driver limit. Returns
    /// `Ok(())` if safe, or `Err` with a descriptive message. Call before
    /// every `vkAllocateMemory`.
    pub fn check_allocatable(&self, label: &str) -> Result<(), String> {
        let count = self.allocation_count.get();
        if count >= self.max_allocations {
            return Err(format!(
                "VkDeviceMemory limit reached ({count}/{max}): {label}",
                max = self.max_allocations,
            ));
        }
        Ok(())
    }

    /// Record a successful allocation.
    pub fn track_alloc(&self) {
        self.allocation_count.set(self.allocation_count.get() + 1);
    }

    /// Record a freed allocation.
    pub fn track_free(&self) {
        let c = self.allocation_count.get();
        self.allocation_count.set(c.saturating_sub(1));
    }

    /// Allocate device memory with budget checking. Wraps `vkAllocateMemory`
    /// and automatically tracks the allocation count.
    pub fn allocate_memory(
        &self,
        create_info: &vk::MemoryAllocateInfo,
        label: &str,
    ) -> Result<vk::DeviceMemory, vk::Result> {
        self.check_allocatable(label).map_err(|e| {
            error!("vulkan: {e}");
            vk::Result::ERROR_OUT_OF_DEVICE_MEMORY
        })?;
        let mem = unsafe { self.device.allocate_memory(create_info, None) }?;
        self.track_alloc();
        Ok(mem)
    }

    /// Free device memory and update the budget counter.
    pub fn free_memory(&self, memory: vk::DeviceMemory) {
        unsafe { self.device.free_memory(memory, None) };
        self.track_free();
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("no graphics queue family")]
    NoGraphicsQueue,
    #[error("missing required extension: {0}")]
    MissingExtension(String),
    #[error("vkCreateDevice failed: {0}")]
    Create(String),
}

pub fn create(phd: &PhysicalDevice) -> Result<VulkanDevice, DeviceError> {
    for ext in required_extensions() {
        if !phd.has_device_extension(ext) {
            return Err(DeviceError::MissingExtension(
                ext.to_string_lossy().into_owned(),
            ));
        }
    }

    let instance = phd.instance().handle();
    let queue_family_index = unsafe {
        instance
            .get_physical_device_queue_family_properties(phd.handle())
            .iter()
            .position(|q| q.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .ok_or(DeviceError::NoGraphicsQueue)? as u32
    };

    let priorities = [1.0f32];
    let queue_info = vk::DeviceQueueCreateInfo::default()
        .queue_family_index(queue_family_index)
        .queue_priorities(&priorities);
    let ext_ptrs: Vec<*const c_char> = required_extensions().iter().map(|e| e.as_ptr()).collect();

    let mut features12 =
        vk::PhysicalDeviceVulkan12Features::default().timeline_semaphore(true);
    let mut features13 = vk::PhysicalDeviceVulkan13Features::default()
        .dynamic_rendering(true)
        .synchronization2(true);

    // Enable anisotropic sampling when the device advertises it, so the world anti-aliasing
    // `aniso` composite sampler is legal. No cost when unused; skipped on the
    // rare device that lacks it (the composite path falls back to isotropic).
    let supported = unsafe { instance.get_physical_device_features(phd.handle()) };
    let mut base_features = vk::PhysicalDeviceFeatures::default();
    if supported.sampler_anisotropy == vk::TRUE {
        base_features = base_features.sampler_anisotropy(true);
    }

    let create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(std::slice::from_ref(&queue_info))
        .enabled_extension_names(&ext_ptrs)
        .enabled_features(&base_features)
        .push_next(&mut features12)
        .push_next(&mut features13);

    let device = unsafe {
        instance
            .create_device(phd.handle(), &create_info, None)
            .map_err(|e| DeviceError::Create(format!("{e}")))?
    };

    info!("vulkan logical device created (queue family {queue_family_index})");
    let max_allocations = unsafe {
        instance
            .get_physical_device_properties(phd.handle())
            .limits
            .max_memory_allocation_count
    };
    Ok(VulkanDevice {
        device,
        queue_family_index,
        instance: instance.clone(),
        max_allocations,
        allocation_count: std::cell::Cell::new(0),
    })
}

/// Find a memory type index that satisfies both the resource's `type_bits`
/// bitmask and the required property flags. Returns `None` if no type matches.
pub fn find_memory_type(
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

/// Convenience wrapper: accepts a [`VulkanDevice`] + smithay [`PhysicalDevice`].
pub fn find_memory_type_for(
    dev: &VulkanDevice,
    phd: &PhysicalDevice,
    type_bits: u32,
    props: vk::MemoryPropertyFlags,
) -> Option<u32> {
    find_memory_type(&dev.instance, phd.handle(), type_bits, props)
}
