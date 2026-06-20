//! One attribute's preference state.

use std::any::{Any, TypeId};
use std::sync::Arc;

/// One attribute's preference state. Default: `enabled = true`,
/// `override_value = None` — "no override; use the inferred best value."
pub struct PreferenceField {
    pub enabled: bool,
    pub override_value: Option<Arc<dyn Any + Send + Sync>>,
    pub override_type_id: Option<TypeId>,
}

impl std::fmt::Debug for PreferenceField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreferenceField")
            .field("enabled", &self.enabled)
            .field("has_override", &self.override_value.is_some())
            .finish()
    }
}

impl Clone for PreferenceField {
    fn clone(&self) -> Self {
        Self {
            enabled: self.enabled,
            override_value: self.override_value.clone(),
            override_type_id: self.override_type_id,
        }
    }
}

impl Default for PreferenceField {
    fn default() -> Self {
        Self {
            enabled: true,
            override_value: None,
            override_type_id: None,
        }
    }
}

impl PreferenceField {
    pub fn has_override(&self) -> bool {
        self.override_value.is_some()
    }
}
