//! The per-surface cache map and accessor.

use compositor_kernel_vulkan_texture_image_base::VulkanTexture;
use smithay::backend::renderer::ContextId;
use smithay::wayland::compositor::SurfaceData;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Per-surface SHM textures, keyed by renderer `ContextId` (so multi-GPU setups
/// cache one texture per renderer). Stored in the surface's `data_map`.
pub type ShmCacheMap = HashMap<ContextId<VulkanTexture>, VulkanTexture>;

/// Get (or lazily create) the SHM texture cache stored in this surface's
/// `data_map`. Returns an owned `Arc` so the caller can lock it without holding
/// a borrow of the surface data.
pub fn for_surface(surface: &SurfaceData) -> Arc<Mutex<ShmCacheMap>> {
    surface
        .data_map
        .get_or_insert_threadsafe(|| Arc::new(Mutex::new(ShmCacheMap::new())))
        .clone()
}
