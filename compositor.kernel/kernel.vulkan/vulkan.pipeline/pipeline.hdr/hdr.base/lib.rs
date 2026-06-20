//! HDR composite pipeline (M5), extracted from `renderer.core` so the HDR path
//! — and its naga build-dependency — lives beside the other composite pipelines
//! under `vulkan.pipeline`, not in the renderer assembly crate.

pub mod hdr;
