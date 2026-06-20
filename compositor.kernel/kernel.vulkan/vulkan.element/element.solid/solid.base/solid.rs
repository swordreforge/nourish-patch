//! Solid-color element drawing. Phase 4 Step 3 — real; exercised by the
//! renderer self-test's recorded solid frame (the sentinel mirror left with
//! the gles sentinel in the completion pass — it was dead in the live path).

use ash::vk;
use compositor_kernel_vulkan_device_factory_base::factory::VulkanDevice;
use compositor_kernel_vulkan_pipeline_composite_base::composite::{
    self, CompositePipelines, PushQuad,
};

pub fn quad(output_size: (u32, u32), dst: (i32, i32, i32, i32), color: [f32; 4]) -> PushQuad {
    let (ow, oh) = (output_size.0 as f32, output_size.1 as f32);
    PushQuad {
        dst: [
            (dst.0 as f32 / ow) * 2.0 - 1.0,
            (dst.1 as f32 / oh) * 2.0 - 1.0,
            (dst.2 as f32 / ow) * 2.0,
            (dst.3 as f32 / oh) * 2.0,
        ],
        src: [0.0, 0.0, 1.0, 1.0],
        color,
    }
}

pub fn draw(
    device: &VulkanDevice,
    pipelines: &CompositePipelines,
    cmd: vk::CommandBuffer,
    push: PushQuad,
) {
    composite::draw_solid(device, pipelines, cmd, push);
}
