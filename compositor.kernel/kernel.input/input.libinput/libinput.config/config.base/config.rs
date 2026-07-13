//! Per-device settings applied on device-added. Currently only tap-to-click
//! is enabled; additional per-device input configuration (natural scroll,
//! accel profile, …) can be added here as the seam requires.

use smithay::reexports::input::Device;

/// Per-device input settings applied on `DeviceAdded`.
///
/// Currently hardcoded: tap-to-click enabled by default. When a settings UI
/// exists, the fields here can be read from preferences instead.
#[derive(Debug, Clone)]
pub struct DeviceSettings {
    /// Whether tap-to-click is enabled (touchpad only).
    pub tap_to_click: bool,
}

impl Default for DeviceSettings {
    fn default() -> Self {
        Self { tap_to_click: true }
    }
}

/// Apply per-device libinput configuration when a device is added.
///
/// Detects touchpads via [`Device::config_tap_finger_count`] and enables
/// tap-to-click when the setting is active.
pub fn on_device_added(device: &mut Device, settings: &DeviceSettings) {
    if device.config_tap_finger_count() > 0 {
        let name = device.name().to_string();
        if settings.tap_to_click {
            let _ = device
                .config_tap_set_enabled(true)
                .inspect_err(|e| {
                    warn!("tap-to-click not available on {name}: {e:?}");
                });
        }
        info!("touchpad configured: {name} (tap-to-click={})", settings.tap_to_click);
    }
}
