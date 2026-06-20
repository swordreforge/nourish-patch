use compositor_monitor_compositor_iced_base::IcedHandle;

pub struct State {
    pub handle: Option<IcedHandle<compositor_monitor_launcher_ui_base::Launcher>>,

}


impl State {
    pub fn new() -> Self {
        return Self {
            handle: None
        }
    }
}