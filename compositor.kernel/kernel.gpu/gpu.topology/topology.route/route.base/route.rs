//! Cross-GPU copy/routing decisions. Single-node era: no copies.

use smithay::backend::drm::DrmNode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyRoute {
    /// Render node and scanout node are the same device: direct.
    None,
    /// Cross-GPU: render on one node, dmabuf-copy toward the scanout node.
    DmabufCopy { render: DrmNode, scanout: DrmNode },
}

pub fn route(render: DrmNode, scanout: DrmNode) -> CopyRoute {
    if render == scanout || render.dev_id() == scanout.dev_id() {
        CopyRoute::None
    } else {
        CopyRoute::DmabufCopy { render, scanout }
    }
}
