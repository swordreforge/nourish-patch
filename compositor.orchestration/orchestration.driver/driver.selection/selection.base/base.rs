use compositor_support_system_storage_token_base::base::{Token, TokenMut};
use compositor_monitor_compositor_iced_base::HandleId;

/// Selection-overlay driver data: the live iced toolbar instance (created when
/// the selection becomes non-empty, destroyed when it empties) plus the
/// last-seen selection count used to gate redundant UI dispatches. The handle
/// is shared between the render-path reconciler (create/destroy/count) and the
/// `SelectSystem`-driven reposition (see `compositor_y5_select_overlay_system`).
pub struct SelectionOverlayState {
    /// The live toolbar instance, if one is currently shown.
    pub handle: Option<HandleId>,
    /// Last selection size pushed to the UI (avoids redundant dispatches).
    pub count: i32,
}

impl Default for SelectionOverlayState {
    fn default() -> Self {
        Self { handle: None, count: 0 }
    }
}

pub static SELECTION_OVERLAY: Token<SelectionOverlayState> = Token::new();
pub static SELECTION_OVERLAY_MUT: TokenMut<SelectionOverlayState> =
    TokenMut::new(&SELECTION_OVERLAY);

/// Where the selection toolbar is placed. Compile-time knob, shared by the
/// reconciler (placement at create) and the reposition system.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Fixed at the bottom-center of the screen (zoom-independent).
    ScreenBottomCenter,
    /// World-space, centered just below the cursor, above all windows. Scales
    /// with camera zoom and re-anchors to the cursor on each selection change.
    WorldAtCursor,
}

/// The active placement. Flip this constant to switch modes.
pub const SELECTION_OVERLAY_PLACEMENT: Placement = Placement::WorldAtCursor;

/// Toolbar size in physical pixels (its native, unzoomed size).
pub const BAR_W: i32 = 440;
pub const BAR_H: i32 = 120;
/// Gap below the screen bottom (ScreenBottomCenter).
pub const SCREEN_BOTTOM_MARGIN: i32 = 100;
/// Gap below the cursor, in world-physical px (WorldAtCursor).
pub const CURSOR_DY: f64 = 12.0;
