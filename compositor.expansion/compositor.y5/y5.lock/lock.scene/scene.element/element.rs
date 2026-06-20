use smithay::backend::renderer::{ImportAll, ImportMem};
use compositor_orchestration_draw_dispatch_frame::SceneDispatch;
use compositor_monitor_compositor_iced_base::IcedRenderElement;

smithay::render_elements! {
    pub LockSceneElement<R> where R: ImportAll + ImportMem + SceneDispatch;
    Surface = IcedRenderElement,
    Pointer = compositor_orchestration_seat_pointer_element::element::PointerRenderElement<R>,
    Background2D = compositor_background_two_draw_element::element::ParallaxBackground,
    Background3D = compositor_support_bevy_core_compositor_base::BevyRenderElement,
    // Renderer-native texture imported from the iced/bevy dmabuf (Vulkan path),
    // same as the main scene's SceneElement::Texture.
    Texture = compositor_orchestration_draw_scene_element::element::PreImported<R>,
}
