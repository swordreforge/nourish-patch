//! `BevyItem`: type-erased instance entry stored by the registry.

use std::any::TypeId;

use compositor_support_bevy_core_context_base::WgpuVulkanContext;
use compositor_support_bevy_core_element_base::BevyRenderElement;
use compositor_support_bevy_core_fault_base::SurfaceError;
use compositor_support_bevy_core_handle_base::HandleId;
use compositor_support_bevy_core_instance_base::{BevyInstance, BevyInstanceAny};
use compositor_support_bevy_core_scene_base::BevyScene;
use compositor_support_bevy_core_space_base::{BevySpace, Transform, item_screen_location, item_screen_size};
use smithay::backend::renderer::element::Id;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::utils::CommitCounter;
use smithay::utils::{Physical, Point, Rectangle, Size};

pub struct BevyItem {
    pub inner: Box<dyn BevyInstanceAny>,
    pub layer: u64,
    type_id: TypeId,
    space: BevySpace,
}

impl BevyItem {
    #[doc(hidden)]
    pub fn new<S: BevyScene>(inst: BevyInstance<S>, space: BevySpace, layer: u64) -> Self {
        Self { type_id: TypeId::of::<BevyInstance<S>>(), inner: Box::new(inst), space, layer }
    }

    pub fn handle_id(&self) -> HandleId { self.inner.handle_id() }
    pub fn space(&self) -> BevySpace { self.space }
    pub fn set_space(&mut self, space: BevySpace) {
        if self.space != space { self.space = space; self.inner.bump_commit(); }
    }

    pub fn location(&self) -> Point<i32, Physical> { self.inner.location() }
    pub fn size(&self) -> Size<i32, Physical> { self.inner.size() }
    pub fn set_location(&mut self, p: Point<i32, Physical>) { self.inner.set_location(p); }
    pub fn screen_location(&self, transform: &Transform, output_size: Size<f64, Physical>) -> Point<i32, Physical> {
        item_screen_location(self.space, transform, output_size, self.inner.location())
    }

    pub fn screen_size(&self, transform: &Transform) -> Size<i32, Physical> {
        item_screen_size(self.space, transform, self.inner.size())
    }

    pub fn screen_rect(&self, transform: &Transform, output_size: Size<f64, Physical>) -> Rectangle<i32, Physical> {
        Rectangle::from_loc_and_size(self.screen_location(transform, output_size), self.screen_size(transform))
    }

    pub fn contains_screen_point(&self, point: Point<f64, Physical>, transform: &Transform, output_size: Size<f64, Physical>) -> bool {
        let r = self.screen_rect(transform, output_size);
        let r = Rectangle::<f64, Physical>::from_loc_and_size((r.loc.x as f64, r.loc.y as f64), (r.size.w as f64, r.size.h as f64));
        r.contains(point)
    }

    pub fn request_resize(&mut self, new_size: Size<i32, Physical>, scale_factor: f32) { self.inner.request_resize(new_size, scale_factor); }
    pub fn pending_resize(&self) -> Option<(Size<i32, Physical>, f32)> { self.inner.pending_resize() }
    pub fn commit(&self) -> CommitCounter { self.inner.commit() }
    pub fn is<S: BevyScene>(&self) -> bool { self.type_id == TypeId::of::<BevyInstance<S>>() }

    pub fn get<S: BevyScene>(&self) -> Option<&BevyInstance<S>> {
        if self.is::<S>() { self.inner.as_any().downcast_ref::<BevyInstance<S>>() } else { None }
    }

    pub fn get_mut<S: BevyScene>(&mut self) -> Option<&mut BevyInstance<S>> {
        if self.is::<S>() { self.inner.as_any_mut().downcast_mut::<BevyInstance<S>>() } else { None }
    }

    #[doc(hidden)] pub fn smithay_id(&self) -> &Id { self.inner.smithay_id() }
    #[doc(hidden)] pub fn tick(&mut self) { self.inner.tick(); }
    #[doc(hidden)] pub fn bump_commit(&mut self) { self.inner.bump_commit(); }

    #[doc(hidden)]
    pub fn element_in(&self, transform: &Transform, output_size: Size<f64, Physical>) -> BevyRenderElement {
        let location = self.screen_location(transform, output_size);
        let world_zoom = match self.space { BevySpace::Screen => 1.0, BevySpace::World => transform.zoom };
        BevyRenderElement {
            texture: self.inner.texture_handle().clone(),
            dmabuf: self.inner.dmabuf().clone(),
            space: self.space,
            location, size: self.inner.size(), world_zoom,
            id: self.inner.smithay_id().clone(), commit_counter: self.inner.commit(),
        }
    }

    #[doc(hidden)]
    pub fn apply_pending_resize(&mut self, render_node: &str, wgpu_ctx: &WgpuVulkanContext, gles: &mut GlesRenderer) -> Result<bool, SurfaceError> {
        self.inner.apply_pending_resize(render_node, wgpu_ctx, gles)
    }
}

impl std::fmt::Debug for BevyItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BevyItem")
            .field("handle", &self.handle_id()).field("space", &self.space)
            .field("location", &self.location()).field("size", &self.size()).finish()
    }
}
