use smithay::wayland::selection::data_device::DataDeviceState;

pub struct Clipboard {
    // Manages `wl_data_device_manager`. Handles copy/paste (clipboard) and drag-and-drop.
    pub data_device_state: DataDeviceState,
}