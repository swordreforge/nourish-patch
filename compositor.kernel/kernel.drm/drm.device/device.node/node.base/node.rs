//! DrmNode typing; render <-> primary node resolution.
//! (Ex wire.rs `new()` step 2, the node-mapping half.)

use smithay::backend::drm::{DrmNode, NodeType};
use std::path::Path;

/// Resolve a device path to its RENDER node, mirroring the original logic:
/// from_path -> node_with_type(Render), falling back to the node as-is.
pub fn render_node(path: &Path) -> Option<DrmNode> {
    let node = DrmNode::from_path(path).ok()?;
    node.node_with_type(NodeType::Render)
        .and_then(|r| r.ok())
        .or(Some(node))
}

/// The PRIMARY (card) node for a node, if resolvable.
pub fn primary_node(node: DrmNode) -> Option<DrmNode> {
    node.node_with_type(NodeType::Primary).and_then(|r| r.ok())
}

/// Whether a udev dev_t matches this node (or its primary sibling).
pub fn matches_dev(dev_id: u64, node: DrmNode, primary: Option<DrmNode>) -> bool {
    primary.map(|p| dev_id == p.dev_id()).unwrap_or(false) || dev_id == node.dev_id()
}
