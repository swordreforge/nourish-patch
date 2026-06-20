use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::desktop::Window;
use smithay::utils::{Physical, Point, Size};
use compositor_orchestration_core_state_base::Loop;

// Isn't much now. IcedHandle are auto rendered.
pub fn scene(
    state: &mut Loop,
    renderer: &mut GlesRenderer,
    size: Size<i32, Physical>,
    window: &Window,
) {
}