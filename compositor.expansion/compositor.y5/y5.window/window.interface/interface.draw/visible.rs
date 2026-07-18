use compositor_y5_group_state_base::state::Group;
use smithay::desktop::Window;
use compositor_orchestration_core_state_base::Loop;
use compositor_y5_window_interface_record::window::LoopWindow;

pub trait DrawWindow {
    fn visible(&self, _loop: &Loop) -> bool;
}

impl DrawWindow for Window {
    fn visible(&self, _loop: &Loop) -> bool {
        // CHECK: Assume true for visible windows- this visible method is temporary anyway-  dedicated spaces are required.
        let Some(window_uuid) = self.uuid() else {
            return true;
        };

        let Some(group_uuid) = _loop.inner.group().window.get(&window_uuid) else {
            return true;
        };

        let visible = _loop.inner.group().group.get(group_uuid.as_ref())
            .map_or(false, |g| matches!(
                g.Visibility,
                compositor_y5_group_state_base::state::GroupVisibility::Visible(_)
            ));

        visible
    }
}
