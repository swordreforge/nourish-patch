//! Per-surface SHM texture cache. Mirrors the vendored GLES renderer
//! (`gles/mod.rs`), which stores its imported SHM textures in the surface's
//! `data_map` keyed by `ContextId` — so a repeatedly-committed SHM surface
//! reuses its `VkImage` (re-uploading only the damaged region) instead of
//! allocating a fresh image every frame, and the texture's lifetime is tied to
//! the surface (freed when the surface/buffer goes away — no manual eviction).

pub mod cache;
