use smithay::wayland::shm::ShmState;
use compositor_support_smithay_dispatch_state_base::state::{Dispatch, DispatchWire};

/// Handles Shared Memory (`wl_shm`).
///
/// SHM is the fallback (and default) way clients send pixels to the compositor.
/// The client allocates memory in RAM, writes raw pixel bytes (like ARGB8888) to it,
/// and sends a file descriptor over the Wayland socket.
pub fn shm_state(dispatch: &Dispatch) -> &ShmState {
    // By returning this state, Smithay automatically handles the `mmap` (memory mapping)
    // of the file descriptors the client sends, turning them into readable Rust slices
    // that your renderer can upload to the screen.
    &dispatch.shm.state
}
