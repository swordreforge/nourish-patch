//! The per-format pipeline caches (SDR composite + background, HDR composite +
//! background), created on demand and held for the renderer's lifetime.

use ash::vk;

use crate::error::VulkanError;
use super::VulkanRenderer;

impl VulkanRenderer {
    pub(super) fn ensure_pipelines(&mut self, format: vk::Format) -> Result<(), VulkanError> {
        if !self.pipelines.contains_key(&format) {
            let p = compositor_kernel_vulkan_pipeline_composite_base::composite::create(
                &self.dev,
                self.pipeline_cache,
                format,
            )
            .map_err(|e| VulkanError::Vk(format!("composite pipeline: {e}")))?;
            self.pipelines.insert(format, p);
        }
        if !self.background_pipelines.contains_key(&format) {
            let bg = crate::background::BackgroundPipeline::create(&self.dev, format)?;
            self.background_pipelines.insert(format, bg);
            info!("vulkan: native background (parallax HLSL→SPIR-V) pipeline created for {format:?}");
        }
        Ok(())
    }

    /// Lazily build the HDR composite + parallax pipelines for `format`.
    pub(super) fn ensure_hdr_pipeline(&mut self, format: vk::Format) -> Result<(), VulkanError> {
        if self.hdr_pipelines.contains_key(&format) {
            return Ok(());
        }
        let hdr = crate::hdr_composite::HdrComposite::create(
            &self.dev,
            self.phd.handle(),
            self.pipeline_cache,
            format,
        )
        .map_err(|e| VulkanError::Vk(format!("hdr composite pipeline: {e}")))?;
        self.hdr_pipelines.insert(format, hdr);
        let bg = crate::background::HdrBackground::create(&self.dev, format)?;
        self.hdr_background_pipelines.insert(format, bg);
        info!("vulkan: HDR composite + parallax pipelines created for {format:?}");
        Ok(())
    }
}
