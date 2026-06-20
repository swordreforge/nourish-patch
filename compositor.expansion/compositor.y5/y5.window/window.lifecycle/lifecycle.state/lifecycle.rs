use smithay::desktop::Window;
use compositor_y5_window_lifecycle_event::event::WindowLifecycleEvent;

pub struct WindowLifecycle{
    pub incoming: Vec<WindowLifecycleEvent>
}

impl WindowLifecycle {
    pub fn new() -> Self {
        return Self {
            incoming: vec!(),
        }
    }
}