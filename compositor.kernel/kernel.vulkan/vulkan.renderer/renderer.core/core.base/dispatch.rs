//! `VulkanRenderer`'s implementation of the scene-dispatch seam.
//!
//! Blank for now: the GLES-resource elements (iced UI, bevy 3D, parallax pixel
//! shader) carry `GlesTexture`/`GlesPixelProgram` that the Vulkan path cannot
//! consume directly. Per the integration plan, these no-op on Vulkan until each
//! element grows a renderer-native (dmabuf-import / vulkan composite) path. This
//! is the sanctioned "blank draw for non-GLES renderers" hook.

use smithay::backend::renderer::gles::{GlesPixelProgram, GlesTexture, Uniform};
use smithay::utils::{Buffer as BufferCoord, Physical, Rectangle, Size};
use compositor_orchestration_draw_dispatch_frame::{ParallaxUniforms, SceneDispatch};

use crate::background::BackgroundPush;
use crate::error::VulkanError;
use crate::frame::{DrawOp, VulkanFrame};
use crate::renderer::VulkanRenderer;

impl SceneDispatch for VulkanRenderer {
    // Vulkan consumes iced/bevy/parallax output via dmabuf import (PreImported),
    // not the GLES-welded seam below.
    fn prefers_dmabuf() -> bool {
        true
    }

    fn draw_prerendered_texture(
        _frame: &mut VulkanFrame<'_, '_>,
        _texture: &GlesTexture,
        _src: Rectangle<f64, BufferCoord>,
        _dst: Rectangle<i32, Physical>,
        _damage: &[Rectangle<i32, Physical>],
        _alpha: f32,
    ) -> Result<(), VulkanError> {
        // Blank until a dmabuf-imported vulkan texture path lands for these elements.
        Ok(())
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
        vk: ParallaxUniforms,
    ) -> Result<(), VulkanError> {
        // Native Vulkan parallax: queue a fullscreen background draw with the
        // shader's push constants (the GLES program/uniforms are unused here).
        // Replayed by the background pipeline during `submit_frame`.
        let push = BackgroundPush {
            res_zoom_time: [vk.resolution[0], vk.resolution[1], vk.zoom, vk.time],
            pan_flow: [vk.pan[0], vk.pan[1], vk.flow_offset[0], vk.flow_offset[1]],
            lock_alpha: [vk.lock_amount, vk.alpha, 0.0, 0.0],
        };
        frame.ops.push(DrawOp::Parallax { push });
        Ok(())
    }
}
