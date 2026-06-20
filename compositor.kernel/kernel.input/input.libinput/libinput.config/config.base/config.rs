//! Per-device settings applied on device-added. Designated home (seam):
//! today no per-device configuration is applied; the entry exists so
//! `native.device/device.activate` has a stable call target.

use smithay::reexports::input::Device;

#[derive(Debug, Clone, Default)]
pub struct DeviceSettings {
    // Populated when per-device input configuration becomes a feature
    // (tap-to-click, natural scroll, accel profile, ...).
}

pub fn on_device_added(device: &Device, _settings: &DeviceSettings) {
    trace!(
        "input device added (no settings applied): {}",
        device.name()
    );
}
