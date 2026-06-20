//! Typed renderer preference. Self-contained value; population out of scope.
//! (Pixman dropped from the project — gles and vulkan are the peers.)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererKind {
    Gles,
    Vulkan,
}

#[derive(Debug, Clone)]
pub struct RendererRank {
    /// Renderers in selection order; the first entry is built. There is no
    /// fallback between renderers: a selected renderer that cannot run is a
    /// configuration failure and panics at assembly.
    pub order: Vec<RendererKind>,
}

impl Default for RendererRank {
    fn default() -> Self {
        Self { order: vec![RendererKind::Gles] }
    }
}

pub fn get() -> RendererRank {
    RendererRank::default()
}
