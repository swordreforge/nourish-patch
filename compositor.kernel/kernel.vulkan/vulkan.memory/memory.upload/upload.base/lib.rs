//! CPU → VkImage upload for SHM / memory client buffers — extracted from
//! `renderer.core`. Provides:
//! - [`upload::create_and_upload`]: allocate a device-local SAMPLED image and
//!   fill it from a tightly-packed RGBA buffer (the first import of a buffer).
//! - [`upload::update_region`]: re-upload a sub-region into an EXISTING image,
//!   so a repeatedly-committed SHM surface (the common case) reuses its image
//!   instead of allocating a fresh one every frame. Backs both
//!   `ImportMemWl::import_shm_buffer`'s per-surface reuse and
//!   `ImportMem::update_memory`.
//! - [`upload::StagingBuffer`]: a renderer-owned host-visible staging buffer
//!   reused across uploads (grows on demand), so steady-state SHM updates
//!   allocate no new host memory.

pub mod upload;
