//! Native Vulkan background pipeline: a fullscreen-triangle pass that runs the
//! parallax space shader (HLSL → SPIR-V via glslang, embedded below). This is
//! the Vulkan counterpart of the GLES pixel-shader background — it exercises a
//! real `VkPipeline` + fragment shader rather than importing a GLES result.
//!
//! No vertex buffers (positions from `SV_VertexID`), no descriptor sets;
//! animation/camera state arrives in a 48-byte push constant (3×float4).
//!
//! Recompile the embedded SPIR-V (sources `shaders/*.hlsl` beside this file):
//! ```text
//! glslangValidator -V -D -e main -S vert --target-env vulkan1.3 \
//!     shaders/background.vert.hlsl -o shaders/background.vert.spv
//! glslangValidator -V -D -e main -S frag --target-env vulkan1.3 \
//!     shaders/parallax.frag.hlsl  -o shaders/parallax.frag.spv
//! ```

use ash::vk;
use compositor_kernel_vulkan_device_factory_base::factory::VulkanDevice;

use compositor_kernel_vulkan_renderer_error_base::VulkanError;

pub const BG_VERT_SPV: &[u8] = include_bytes!("shaders/background.vert.spv");
pub const BG_FRAG_SPV: &[u8] = include_bytes!("shaders/parallax.frag.spv");

/// Push constants for the parallax shader (matches the HLSL `PushData`: three
/// float4s = 48 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BackgroundPush {
    /// xy = resolution, z = zoom, w = time
    pub res_zoom_time: [f32; 4],
    /// xy = pan, zw = flow_offset
    pub pan_flow: [f32; 4],
    /// x = lock_amount, y = alpha
    pub lock_alpha: [f32; 4],
}

pub struct BackgroundPipeline {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub color_format: vk::Format,
}

fn shader_module(dev: &ash::Device, spv: &[u8]) -> Result<vk::ShaderModule, VulkanError> {
    let words: Vec<u32> = spv
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let info = vk::ShaderModuleCreateInfo::default().code(&words);
    unsafe {
        dev.create_shader_module(&info, None)
            .map_err(|e| VulkanError::Vk(format!("bg shader module: {e}")))
    }
}

impl BackgroundPipeline {
    pub fn create(device: &VulkanDevice, color_format: vk::Format) -> Result<Self, VulkanError> {
        let dev = &device.device;

        let push_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(std::mem::size_of::<BackgroundPush>() as u32);
        let layout_info =
            vk::PipelineLayoutCreateInfo::default().push_constant_ranges(std::slice::from_ref(&push_range));
        let layout = unsafe {
            dev.create_pipeline_layout(&layout_info, None)
                .map_err(|e| VulkanError::Vk(format!("bg pipeline layout: {e}")))?
        };

        let vert = shader_module(dev, BG_VERT_SPV)?;
        let frag = shader_module(dev, BG_FRAG_SPV)?;
        let entry = std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap();
        let stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert)
                .name(entry),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag)
                .name(entry),
        ];

        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);
        let raster = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE)
            .line_width(1.0);
        let multisample = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
        // Premultiplied-alpha over (the shader emits color*alpha; same blend as
        // the composite pipeline so it sits correctly under the scene).
        let blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(vk::BlendOp::ADD)
            .color_write_mask(vk::ColorComponentFlags::RGBA);
        let blend = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(std::slice::from_ref(&blend_attachment));
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let color_formats = [color_format];
        let mut rendering_info =
            vk::PipelineRenderingCreateInfo::default().color_attachment_formats(&color_formats);
        let info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&raster)
            .multisample_state(&multisample)
            .color_blend_state(&blend)
            .dynamic_state(&dynamic)
            .layout(layout)
            .push_next(&mut rendering_info);
        let pipeline = unsafe {
            dev.create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&info), None)
                .map_err(|(_, e)| VulkanError::Vk(format!("bg graphics pipeline: {e}")))?[0]
        };

        unsafe {
            dev.destroy_shader_module(vert, None);
            dev.destroy_shader_module(frag, None);
        }

        Ok(Self {
            pipeline,
            layout,
            color_format,
        })
    }

    /// Draw the fullscreen background into the active rendering pass.
    pub fn draw(&self, device: &VulkanDevice, cmd: vk::CommandBuffer, push: &BackgroundPush) {
        let dev = &device.device;
        let bytes = unsafe {
            std::slice::from_raw_parts(
                (push as *const BackgroundPush) as *const u8,
                std::mem::size_of::<BackgroundPush>(),
            )
        };
        unsafe {
            dev.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
            dev.cmd_push_constants(cmd, self.layout, vk::ShaderStageFlags::FRAGMENT, 0, bytes);
            dev.cmd_draw(cmd, 3, 1, 0, 0);
        }
    }

    pub fn destroy(&self, device: &VulkanDevice) {
        unsafe {
            device.device.destroy_pipeline(self.pipeline, None);
            device.device.destroy_pipeline_layout(self.layout, None);
        }
    }
}

// ── HDR parallax (M5) ────────────────────────────────────────────────────────

/// Push constants for the HDR parallax shader (`parallax_hdr.wgsl`): the SDR
/// fields plus the HDR levels. 4×vec4 = 64 bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HdrBackgroundPush {
    pub res_zoom_time: [f32; 4],
    pub pan_flow: [f32; 4],
    pub lock_alpha: [f32; 4],
    /// x = sdr_white_nits, y = max_nits, z/w reserved.
    pub hdr: [f32; 4],
}

/// HDR-graded parallax background pipeline — used only on the HDR path; the SDR
/// `BackgroundPipeline` above is untouched. Shader is `parallax_hdr.wgsl`
/// (naga-compiled at build time).
pub struct HdrBackground {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub color_format: vk::Format,
}

const PARALLAX_HDR_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/parallax_hdr.spv"));

impl HdrBackground {
    pub fn create(device: &VulkanDevice, color_format: vk::Format) -> Result<Self, VulkanError> {
        let dev = &device.device;

        let push_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(std::mem::size_of::<HdrBackgroundPush>() as u32);
        let layout = unsafe {
            dev.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default()
                    .push_constant_ranges(std::slice::from_ref(&push_range)),
                None,
            )
            .map_err(|e| VulkanError::Vk(format!("hdr bg pipeline layout: {e}")))?
        };

        let module = shader_module(dev, PARALLAX_HDR_SPV)?;
        let vs = std::ffi::CStr::from_bytes_with_nul(b"vs_main\0").unwrap();
        let fs = std::ffi::CStr::from_bytes_with_nul(b"fs_main\0").unwrap();
        let stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(module)
                .name(vs),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(module)
                .name(fs),
        ];

        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);
        let raster = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE)
            .line_width(1.0);
        let multisample = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
        let blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(vk::BlendOp::ADD)
            .color_write_mask(vk::ColorComponentFlags::RGBA);
        let blend = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(std::slice::from_ref(&blend_attachment));
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let color_formats = [color_format];
        let mut rendering_info =
            vk::PipelineRenderingCreateInfo::default().color_attachment_formats(&color_formats);
        let info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&raster)
            .multisample_state(&multisample)
            .color_blend_state(&blend)
            .dynamic_state(&dynamic)
            .layout(layout)
            .push_next(&mut rendering_info);
        let pipeline = unsafe {
            dev.create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&info), None)
                .map_err(|(_, e)| VulkanError::Vk(format!("hdr bg graphics pipeline: {e}")))?[0]
        };
        unsafe { dev.destroy_shader_module(module, None) };

        Ok(Self {
            pipeline,
            layout,
            color_format,
        })
    }

    pub fn draw(&self, device: &VulkanDevice, cmd: vk::CommandBuffer, push: &HdrBackgroundPush) {
        let dev = &device.device;
        let bytes = unsafe {
            std::slice::from_raw_parts(
                (push as *const HdrBackgroundPush) as *const u8,
                std::mem::size_of::<HdrBackgroundPush>(),
            )
        };
        unsafe {
            dev.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
            dev.cmd_push_constants(
                cmd,
                self.layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                bytes,
            );
            dev.cmd_draw(cmd, 3, 1, 0, 0);
        }
    }

    pub fn destroy(&self, device: &VulkanDevice) {
        unsafe {
            device.device.destroy_pipeline(self.pipeline, None);
            device.device.destroy_pipeline_layout(self.layout, None);
        }
    }
}
