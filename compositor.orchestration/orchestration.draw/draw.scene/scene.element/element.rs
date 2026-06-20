use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::{ImportAll, ImportMem};
use compositor_orchestration_draw_dispatch_frame::SceneDispatch;
use compositor_orchestration_seat_pointer_element::element::PointerRenderElement;
use compositor_monitor_compositor_iced_base::IcedRenderElement;

pub use compositor_orchestration_draw_scene_preimported::preimported::PreImported;

smithay::render_elements! {
    pub SceneElement<R> where R: ImportAll + ImportMem + SceneDispatch;
    Canvas = compositor_y5_canvas_draw_element::element::Element<R>,
    Layershell = WaylandSurfaceRenderElement<R>,
    Surface = IcedRenderElement,
    Pointer = PointerRenderElement<R>,
    Background2D = compositor_background_two_draw_element::element::ParallaxBackground,
    Background3D = compositor_support_bevy_core_compositor_base::BevyRenderElement,
    Texture = PreImported<R>,
    Sentinel = SolidColorRenderElement,
}
