//! Per-device settings applied on device-added. Currently enables tap-to-click
//! with configurable button map, drag, and drag-lock. Additional per-device
//! input configuration (natural scroll, accel profile, …) can be added here
//! as the seam requires.

use smithay::reexports::input::{
    Device, DragLockState, TapButtonMap,
};

/// Per-device input settings applied on `DeviceAdded`.
///
/// Has safe defaults: tap-to-click on, drag on, others off. When a settings UI
/// exists, the fields here can be read from preferences instead.
///
/// # `Option` fields
///
/// `None` = do **not** touch this setting (libinput default left intact).
#[derive(Debug, Clone)]
pub struct DeviceSettings {
    /// Whether tap-to-click is enabled (touchpad only).
    pub tap_to_click: bool,

    /// 1/2/3 finger tap → left/right/middle mapping. `None` → libinput default.
    pub tap_button_map: Option<TapButtonMap>,

    /// Whether tap-and-hold starts a drag. `None` → libinput default.
    pub tap_drag_enabled: Option<bool>,

    /// Drag lock (tap to hold, tap again to release). `None` → libinput default.
    pub tap_drag_lock_enabled: Option<DragLockState>,
}

impl Default for DeviceSettings {
    fn default() -> Self {
        Self {
            tap_to_click: true,
            tap_button_map: None,
            tap_drag_enabled: Some(true),
            tap_drag_lock_enabled: None,
        }
    }
}

/// Apply per-device libinput configuration when a device is added.
///
/// Detects touchpads via [`Device::config_tap_finger_count`] and applies every
/// setting from `settings` whose value is `Some(…)`.
pub fn on_device_added(device: &mut Device, settings: &DeviceSettings) {
    if device.config_tap_finger_count() == 0 {
        return;
    }
    let name = device.name().to_string();

    if settings.tap_to_click {
        let _ = device
            .config_tap_set_enabled(true)
            .inspect_err(|e| {
                warn!("tap-to-click not available on {name}: {e:?}");
            });
    }
    if let Some(map) = settings.tap_button_map {
        let _ = device
            .config_tap_set_button_map(map)
            .inspect_err(|e| {
                warn!("tap-button-map not available on {name}: {e:?}");
            });
    }
    if let Some(enabled) = settings.tap_drag_enabled {
        let _ = device
            .config_tap_set_drag_enabled(enabled)
            .inspect_err(|e| {
                warn!("tap-drag not available on {name}: {e:?}");
            });
    }
    if let Some(state) = settings.tap_drag_lock_enabled {
        let _ = device
            .config_tap_set_drag_lock_enabled(state)
            .inspect_err(|e| {
                warn!("tap-drag-lock not available on {name}: {e:?}");
            });
    }

    info!(
        "touchpad configured: {name} \
         (tap-to-click={}, tap-button-map={}, tap-drag={}, tap-drag-lock={})",
        settings.tap_to_click,
        settings.tap_button_map.map_or("default", |m| match m {
            TapButtonMap::LeftRightMiddle => "LRM",
            TapButtonMap::LeftMiddleRight => "LMR",
            _ => "?",
        }),
        settings.tap_drag_enabled.map_or("default", |v| if v { "on" } else { "off" }),
        settings.tap_drag_lock_enabled
            .map_or("default", |s| match s {
                DragLockState::Disabled => "off",
                DragLockState::EnabledTimeout => "timeout",
                _ => "?",
            }),
    );
}
