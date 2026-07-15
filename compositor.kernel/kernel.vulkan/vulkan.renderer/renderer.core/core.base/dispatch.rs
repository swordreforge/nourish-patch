//! `VulkanRenderer`'s implementation of the scene-dispatch seam.
//!
//! When a GLES texture is provided, we attempt to re-use its underlying dmabuf
//! (if it was created via dmabuf import) and import it into Vulkan. This enables
//! zero-copy sharing between GLES and Vulkan renderers.

use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::renderer::gles::{GlesPixelProgram, GlesTexture, Uniform};
use smithay::backend::renderer::{Frame, ImportDma, Texture};
use smithay::utils::{Buffer as BufferCoord, Physical, Rectangle, Size};
use compositor_orchestration_draw_dispatch_frame::{ElementMeta, NativeShaderPass, SceneDispatch};
use compositor_orchestration_draw_dispatch_frame::ShaderVariant as SeamVariant;

use crate::error::VulkanError;
use crate::frame::{DrawOp, ShaderVariant, VulkanFrame};
use crate::renderer::VulkanRenderer;

/// Copy a seam shader variant into an owned `DrawOp` variant (the push bytes
/// must outlive this dispatch call, so they're copied into the queued op).
fn own_variant(v: SeamVariant<'_>) -> ShaderVariant {
    ShaderVariant {
        id: v.id,
        spv: v.spv.into_owned(),
        vert_spv: v.vert_spv.map(|s| s.into_owned()),
        vert_entry: v.vert_entry.into_owned(),
        frag_entry: v.frag_entry.into_owned(),
        push: v.push.into_owned(),
    }
}

impl SceneDispatch for VulkanRenderer {
    // Vulkan consumes iced/bevy/parallax output via dmabuf import (PreImported),
    // not the GLES-welded seam below.
    fn prefers_dmabuf() -> bool {
        true
    }

    fn set_element_meta(frame: &mut VulkanFrame<'_, '_>, meta: ElementMeta) {
        // Stamp the current element's metadata; `render_texture_from_to` reads it
        // so AA is applied only to world content (windows + iced-world).
        frame.current_meta = meta;
    }

    fn draw_prerendered_texture(
        _frame: &mut VulkanFrame<'_, '_>,
        _texture: &GlesTexture,
        _src: Rectangle<f64, BufferCoord>,
        _dst: Rectangle<i32, Physical>,
        _damage: &[Rectangle<i32, Physical>],
        _alpha: f32,
    ) -> Result<(), VulkanError> {
        // GLES textures cannot be directly rendered by Vulkan.
        // Use draw_prerendered_dmabuf for zero-copy path.
        Ok(())
    }

    fn draw_prerendered_dmabuf(
        frame: &mut VulkanFrame<'_, '_>,
        dmabuf: &smithay::backend::allocator::dmabuf::Dmabuf,
        src: Rectangle<f64, BufferCoord>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        alpha: f32,
    ) -> Result<(), VulkanError> {
        // Import the dmabuf into Vulkan for zero-copy rendering.
        let vk_texture = frame.renderer.import_dmabuf(dmabuf, None)?;
        frame.render_texture_from_to(&vk_texture, src, dst, damage, &[], smithay::utils::Transform::Normal, alpha)
    }

    fn draw_pixel_program(
        frame: &mut VulkanFrame<'_, '_>,
        _program: Option<&GlesPixelProgram>,
        _src: Rectangle<f64, BufferCoord>,
        _dst: Rectangle<i32, Physical>,
        _size: Size<i32, BufferCoord>,
        _damage: &[Rectangle<i32, Physical>],
        _alpha: f32,
        _uniforms: &[Uniform<'_>],
        pass: NativeShaderPass<'_>,
    ) -> Result<(), VulkanError> {
        // Native fullscreen shader: queue a generic shader pass carrying the
        // producer's SPIR-V + push bytes (the GLES program/uniforms are unused
        // here). Replayed by a `FullscreenPass` during `submit_frame`.
        frame.ops.push(DrawOp::ShaderPass {
            sdr: own_variant(pass.sdr),
            hdr: pass.hdr.map(own_variant),
        });
        Ok(())
    }
}
