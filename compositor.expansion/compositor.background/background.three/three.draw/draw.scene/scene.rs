use smithay::backend::renderer::gles::GlesRenderer;
use smithay::utils::{Physical, Size};
use compositor_support_bevy_core_compositor_base::BevyRenderElement;
use compositor_orchestration_core_state_base::Loop;

pub fn scene(
    state: &mut Loop,
    gles: &mut GlesRenderer,
    output_size: Size<i32, Physical>,
) -> Vec<BevyRenderElement> {
    return vec![];
}
