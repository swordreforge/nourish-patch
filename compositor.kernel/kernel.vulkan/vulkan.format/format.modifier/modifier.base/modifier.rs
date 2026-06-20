//! DRM-format-modifier negotiation — the vulkan side of explicit modifier
//! negotiation (the MODERN path; the Law-7 `framebuffer.modifier` fallback is
//! unrelated and stays off). Phase 4 Step 1 — real via smithay's
//! get_format_modifier_properties.

use ash::vk;
use smithay::backend::allocator::format::FormatSet;
use smithay::backend::allocator::{Format as DrmFormat, Fourcc, Modifier};
use smithay::backend::vulkan::PhysicalDevice;

/// All modifiers the device supports for a format, with their plane counts.
pub fn modifiers(phd: &PhysicalDevice, format: vk::Format) -> Vec<(Modifier, u32)> {
    phd.get_format_modifier_properties(format)
        .map(|props| {
            props
                .into_iter()
                .map(|p| {
                    (
                        Modifier::from(p.drm_format_modifier),
                        p.drm_format_modifier_plane_count,
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Build the render-format set (fourcc x modifier) the vulkan renderer
/// advertises — the input to scanout plane/format negotiation, mirroring what
/// the EGL context provides on the gles path.
pub fn render_formats(phd: &PhysicalDevice, fourccs: &[Fourcc]) -> FormatSet {
    let mut formats: Vec<DrmFormat> = Vec::new();
    for &code in fourccs {
        let Some(vk_fmt) = compositor_kernel_vulkan_format_query_base::query::vk_format(code) else {
            continue;
        };
        if !compositor_kernel_vulkan_format_query_base::query::renderable(phd, vk_fmt) {
            continue;
        }
        for (modifier, _planes) in modifiers(phd, vk_fmt) {
            formats.push(DrmFormat { code, modifier });
        }
    }
    formats.into_iter().collect()
}
