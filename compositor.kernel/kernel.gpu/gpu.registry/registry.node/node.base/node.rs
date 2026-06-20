//! Live GPU node registry: appearance/removal. One node populated today.

use smithay::backend::drm::DrmNode;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct NodeRegistry {
    nodes: HashMap<u64, DrmNode>,
    primary: Option<DrmNode>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, dev_id: u64, node: DrmNode) {
        self.nodes.insert(dev_id, node);
    }

    pub fn remove(&mut self, dev_id: u64) -> Option<DrmNode> {
        let removed = self.nodes.remove(&dev_id);
        if let (Some(r), Some(p)) = (removed.as_ref(), self.primary.as_ref()) {
            if r == p {
                self.primary = None;
            }
        }
        removed
    }

    pub fn set_primary(&mut self, node: DrmNode) {
        self.primary = Some(node);
    }

    pub fn primary(&self) -> Option<DrmNode> {
        self.primary
    }

    pub fn nodes(&self) -> impl Iterator<Item = (&u64, &DrmNode)> {
        self.nodes.iter()
    }

    pub fn contains(&self, dev_id: u64) -> bool {
        self.nodes.contains_key(&dev_id)
    }
}
