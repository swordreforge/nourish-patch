//! Buffer submission + redraw request on the winit window.

use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::winit::WinitGraphicsBackend;
use smithay::utils::{Physical, Rectangle};

pub fn submit(backend: &mut WinitGraphicsBackend<GlesRenderer>, damage: Rectangle<i32, Physical>) {
    backend.submit(Some(&[damage])).unwrap();
}

pub fn request_redraw(backend: &mut WinitGraphicsBackend<GlesRenderer>) {
    backend.window().request_redraw();
}
