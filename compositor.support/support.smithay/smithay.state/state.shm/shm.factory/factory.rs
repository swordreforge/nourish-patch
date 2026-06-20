use smithay::reexports::wayland_server::protocol::wl_shm::{Format, WlShm};
use smithay::reexports::wayland_server::protocol::wl_shm_pool::WlShmPool;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::buffer::BufferHandler;
use smithay::wayland::shm::{ShmHandler, ShmPoolUserData, ShmState};
use compositor_support_smithay_dispatch_state_base::state::DispatchWire;
use compositor_support_smithay_state_shm_base::state::SHMState;

pub fn new<I: DispatchWire>(display_handle: &DisplayHandle) -> SHMState
where
    I: GlobalDispatch<WlShm, GlobalData>
        + Dispatch<WlShm, GlobalData>
        + Dispatch<WlShmPool, ShmPoolUserData>
        + BufferHandler
        + ShmHandler
        + 'static,
{
    // Initialize Shared Memory protocol.
    // We pass `vec![]` for formats because ARGB8888 and XRGB8888 are supported by default.
    // Side-effect: When clients attach SHM buffers to surfaces, `calloop` handles the memory
    // mapping automatically behind the scenes.
    let shm_state = ShmState::new::<I>(
        &display_handle,
        // NEW Added explicit formats
        vec![
            Format::Abgr8888, // common; the byte-swap of Argb
            Format::Xbgr8888,
            Format::Bgr888,
        ],
    );

    return SHMState { state: shm_state };
}
