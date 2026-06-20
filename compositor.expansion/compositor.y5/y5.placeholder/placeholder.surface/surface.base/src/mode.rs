//! Mode enum for [`PlaceholderUi`].

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Compact view: icon + name + Launch / Edit buttons.
    View,
    /// Editor: scrollable attribute form with handler picker.
    Settings,
}

impl Default for Mode {
    fn default() -> Self {
        Self::View
    }
}
