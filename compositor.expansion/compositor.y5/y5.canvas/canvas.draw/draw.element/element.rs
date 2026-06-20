use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::element::{
    Element as SmithayElement, Id, RenderElement, UnderlyingStorage,
};
use smithay::backend::renderer::{ImportAll, ImportMem, Texture};
use compositor_y5_window_draw_element::element::Element as WindowElement;

smithay::render_elements! {
    pub Element<R> where R: ImportAll + ImportMem;
    Window = WindowElement<R>,
    SolidBox = SolidColorRenderElement,
}
