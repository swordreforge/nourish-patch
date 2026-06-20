//! Scene/Lock combined element enum. (Moved from draw.scene/element_combined.rs,
//! imports cleaned.) RETIRED-BY-PLAN: with the frame-plan executor each pass
//! renders its own element list, so the combined enum is only needed by the
//! Locked{pending} fade-in path that composites both lists in one render —
//! exactly where `render.execute` still uses it. It dissolves when that path
//! moves to per-pass rendering.

use compositor_kernel_gles_element_wrap_base::wrap::GlesElementWrapper;
use compositor_y5_lock_scene_element::element::LockSceneElement;
use smithay::backend::drm::DrmDeviceFd;
use smithay::backend::renderer::element::{Element, Id, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::multigpu::gbm::GbmGlesBackend;
use smithay::backend::renderer::multigpu::MultiRenderer;
use smithay::backend::renderer::utils::CommitCounter;
use smithay::backend::renderer::RendererSuper;
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer, Physical, Point, Rectangle, Scale, Transform};
use compositor_orchestration_draw_scene_element::element::SceneElement;

pub enum OutputElement {
    Scene(GlesElementWrapper<SceneElement<GlesRenderer>>),
    Lock(GlesElementWrapper<LockSceneElement<GlesRenderer>>),
}

impl Element for OutputElement {
    fn id(&self) -> &Id {
        match self {
            OutputElement::Scene(e) => e.id(),
            OutputElement::Lock(e) => e.id(),
        }
    }

    fn current_commit(&self) -> CommitCounter {
        match self {
            OutputElement::Scene(e) => e.current_commit(),
            OutputElement::Lock(e) => e.current_commit(),
        }
    }

    fn location(&self, scale: Scale<f64>) -> Point<i32, Physical> {
        match self {
            OutputElement::Scene(e) => e.location(scale),
            OutputElement::Lock(e) => e.location(scale),
        }
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        match self {
            OutputElement::Scene(e) => e.src(),
            OutputElement::Lock(e) => e.src(),
        }
    }

    fn transform(&self) -> Transform {
        match self {
            OutputElement::Scene(e) => e.transform(),
            OutputElement::Lock(e) => e.transform(),
        }
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        match self {
            OutputElement::Scene(e) => e.geometry(scale),
            OutputElement::Lock(e) => e.geometry(scale),
        }
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> smithay::backend::renderer::utils::DamageSet<i32, Physical> {
        match self {
            OutputElement::Scene(e) => e.damage_since(scale, commit),
            OutputElement::Lock(e) => e.damage_since(scale, commit),
        }
    }

    fn opaque_regions(
        &self,
        scale: Scale<f64>,
    ) -> smithay::backend::renderer::utils::OpaqueRegions<i32, Physical> {
        match self {
            OutputElement::Scene(e) => e.opaque_regions(scale),
            OutputElement::Lock(e) => e.opaque_regions(scale),
        }
    }

    fn alpha(&self) -> f32 {
        match self {
            OutputElement::Scene(e) => e.alpha(),
            OutputElement::Lock(e) => e.alpha(),
        }
    }

    fn kind(&self) -> smithay::backend::renderer::element::Kind {
        match self {
            OutputElement::Scene(e) => e.kind(),
            OutputElement::Lock(e) => e.kind(),
        }
    }

    fn is_framebuffer_effect(&self) -> bool {
        match self {
            OutputElement::Scene(e) => e.is_framebuffer_effect(),
            OutputElement::Lock(e) => e.is_framebuffer_effect(),
        }
    }
}

type UdevRenderer<'a> = MultiRenderer<
    'a,
    'a,
    GbmGlesBackend<GlesRenderer, DrmDeviceFd>,
    GbmGlesBackend<GlesRenderer, DrmDeviceFd>,
>;

impl<'a> RenderElement<UdevRenderer<'a>> for OutputElement {
    fn draw(
        &self,
        frame: &mut <UdevRenderer<'a> as RendererSuper>::Frame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), <UdevRenderer<'a> as RendererSuper>::Error> {
        match self {
            OutputElement::Scene(e) => e.draw(frame, src, dst, damage, opaque_regions, cache),
            OutputElement::Lock(e) => e.draw(frame, src, dst, damage, opaque_regions, cache),
        }
    }

    fn underlying_storage(&self, renderer: &mut UdevRenderer<'a>) -> Option<UnderlyingStorage<'_>> {
        match self {
            OutputElement::Scene(e) => e.underlying_storage(renderer),
            OutputElement::Lock(e) => e.underlying_storage(renderer),
        }
    }

    fn capture_framebuffer(
        &self,
        frame: &mut <UdevRenderer<'a> as RendererSuper>::Frame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), <UdevRenderer<'a> as RendererSuper>::Error> {
        match self {
            OutputElement::Scene(e) => e.capture_framebuffer(frame, src, dst, cache),
            OutputElement::Lock(e) => e.capture_framebuffer(frame, src, dst, cache),
        }
    }
}
