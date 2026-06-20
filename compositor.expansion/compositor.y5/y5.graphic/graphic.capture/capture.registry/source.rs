//! What to capture: a full output framebuffer, or a sub-region of one.

use smithay::utils::{Physical, Rectangle};

/// Identifies a compositor output. For multi-monitor support; pass
/// `OutputId(0)` if you have a single output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutputId(pub u64);

/// What this capture reads from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureSource {
    /// The full framebuffer of an output. The registry blits from the
    /// caller-provided framebuffer in `tick`.
    OutputFramebuffer(OutputId),
    /// A sub-region of an output's framebuffer, in physical/output pixels.
    ///
    /// The entry/dmabuf is sized to `rect.size`; the blit copies the
    /// `rect` sub-rect of the composed scene into it. `rect` is mutable over
    /// the capture's lifetime via [`crate::CaptureRegistry::set_region`] — a
    /// move that keeps the same size reuses the entry (only the blit src
    /// changes); a size change reallocates in place (the `EntryId` stays
    /// stable).
    Region {
        output: OutputId,
        rect: Rectangle<i32, Physical>,
    },
}

impl CaptureSource {
    /// The output this source reads from.
    pub fn output(&self) -> OutputId {
        match self {
            CaptureSource::OutputFramebuffer(o) => *o,
            CaptureSource::Region { output, .. } => *output,
        }
    }

    /// The source sub-rect within the output framebuffer, if this source is
    /// a region. `None` means "the whole framebuffer" (the caller supplies
    /// the framebuffer size as the src rect).
    pub fn src_rect(&self) -> Option<Rectangle<i32, Physical>> {
        match self {
            CaptureSource::OutputFramebuffer(_) => None,
            CaptureSource::Region { rect, .. } => Some(*rect),
        }
    }
}
