//! Vulkan instance construction (smithay's vulkan foundation; Phase 4 Step 1).

use smithay::backend::vulkan::version::Version;
use smithay::backend::vulkan::{AppInfo, Instance, InstanceError};

/// The Vulkan API version this compositor targets.
pub fn target_version() -> Version {
    Version::VERSION_1_3
}

pub fn create() -> Result<Instance, InstanceError> {
    let instance = Instance::new(target_version(), None::<AppInfo>)?;
    info!("vulkan instance created (api {:?})", instance.api_version());
    Ok(instance)
}
