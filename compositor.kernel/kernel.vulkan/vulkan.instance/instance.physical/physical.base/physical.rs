//! Physical-device enumeration + DRM-node matching (VK_EXT_physical_device_drm
//! via smithay's PhysicalDevice). Phase 4 Step 1 — real.

use smithay::backend::drm::DrmNode;
use smithay::backend::vulkan::{Instance, PhysicalDevice};

/// All physical devices on the instance.
pub fn enumerate(instance: &Instance) -> Result<Vec<PhysicalDevice>, String> {
    PhysicalDevice::enumerate(instance)
        .map(|iter| iter.collect())
        .map_err(|e| format!("physical device enumeration failed: {e}"))
}

/// The physical device whose primary or render node matches `node` — the
/// bridge between `backend.gpu` selection and the Vulkan world. Nodes cross
/// as values (Law 1).
pub fn for_node(instance: &Instance, node: DrmNode) -> Result<Option<PhysicalDevice>, String> {
    for phd in enumerate(instance)? {
        let primary = phd.primary_node().ok().flatten();
        let render = phd.render_node().ok().flatten();
        let matches = primary.map(|n| n.dev_id() == node.dev_id()).unwrap_or(false)
            || render.map(|n| n.dev_id() == node.dev_id()).unwrap_or(false);
        if matches {
            info!("vulkan physical device matched node: {}", phd.name());
            return Ok(Some(phd));
        }
    }
    Ok(None)
}
