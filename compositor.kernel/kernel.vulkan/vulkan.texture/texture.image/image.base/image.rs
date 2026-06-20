//! `VulkanTexture` — a reference-counted sampled image. The GPU resources live
//! behind an `Arc<TextureInner>` (in the `image.inner` sibling) so smithay can
//! clone `TextureId` freely. Fields/accessors are `pub` so the renderer, the
//! SHM upload path, and the per-surface cache can all share the type.

use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::Texture;
use smithay::utils::{Buffer as BufferCoord, Size};
use std::sync::Arc;

pub use compositor_kernel_vulkan_texture_image_inner::inner::TextureInner;

#[derive(Debug, Clone)]
pub struct VulkanTexture {
    pub inner: Arc<TextureInner>,
    /// Per-surface HDR composite flag `[transfer, is_hdr, 0, 0]`; default SDR.
    pub surf: [f32; 4],
}

impl VulkanTexture {
    pub fn new(inner: Arc<TextureInner>) -> Self {
        Self { inner, surf: [0.0; 4] }
    }
    pub fn view(&self) -> ash::vk::ImageView {
        self.inner.view
    }
    pub fn surf(&self) -> [f32; 4] {
        self.surf
    }
    pub fn set_surf(&mut self, surf: [f32; 4]) {
        self.surf = surf;
    }
}

impl Texture for VulkanTexture {
    fn width(&self) -> u32 {
        self.inner.width
    }
    fn height(&self) -> u32 {
        self.inner.height
    }
    fn size(&self) -> Size<i32, BufferCoord> {
        Size::from((self.inner.width as i32, self.inner.height as i32))
    }
    fn format(&self) -> Option<Fourcc> {
        self.inner.fourcc
    }
}
