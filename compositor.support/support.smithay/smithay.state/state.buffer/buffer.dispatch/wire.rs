use smithay::reexports::wayland_server::protocol::wl_buffer;
use compositor_support_smithay_dispatch_state_base::state::{Dispatch, DispatchWire};

/// Handles the lifecycle of physical pixel buffers.

/// Triggered when a client explicitly destroys a `wl_buffer`.
///
/// Usually, you don't need to manually drop memory here because Smithay handles
/// the underlying mapping. However, if your compositor was caching GPU textures
/// mapped from these buffers, this is where you would evict them from the GPU to free VRAM.
pub fn buffer_destroyed(
    dispatch: &mut Dispatch,
    _buffer: &wl_buffer::WlBuffer,
) {
}
