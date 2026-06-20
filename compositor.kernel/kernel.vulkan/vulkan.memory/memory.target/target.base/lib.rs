//! dmabuf → VkImage import for **render/transfer targets** (rendered or copied
//! into, not sampled) — extracted from `renderer.core`. Mirrors
//! `vulkan.memory/memory.import` (which imports SAMPLED client buffers), but
//! creates a COLOR_ATTACHMENT (bind path) or TRANSFER_DST (capture copy) image,
//! and makes a view only when asked (a TRANSFER_DST-only image cannot have one).

pub mod target;
