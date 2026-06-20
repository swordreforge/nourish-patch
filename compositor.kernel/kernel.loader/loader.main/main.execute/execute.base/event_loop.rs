use smithay::reexports::calloop::EventLoop;
use smithay::reexports::wayland_server::Display;
use compositor_orchestration_core_state_base::Loop;
use compositor_support_smithay_dispatch_state_base::state::Dispatch;

pub fn create<'a>() -> Result<(EventLoop<'a, Loop>, Display<Dispatch>), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::try_new()?;
    let display = Display::new()?;

    Ok((event_loop, display))
}
