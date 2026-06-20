//! The Vulkan renderer's shared error type, extracted into a leaf crate so the
//! renderer piece-crates (memory.target, memory.upload, pipeline.background,
//! capture.blit) can return it without depending on `renderer.core`.

pub mod error;

pub use error::VulkanError;
