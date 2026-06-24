use std::any::Any;

use smithay::utils::{Physical, Point};

use compositor_support_system_buffer_token_base::y5_buffer;
use compositor_support_system_trait_system_base::base::{BufferCx, System, SystemCx, WorldBuilder};
use compositor_y5_select_system_base::base::{SelectionChanged, SELECTION_CHANGED};
use compositor_orchestration_seat_system_pointer::base::POINTER;
use compositor_orchestration_smithay_data_base::data::SCREEN;
use compositor_y5_surface_system_base::base::SURFACE_MUT;
use compositor_orchestration_driver_selection_base::base::{
    BAR_W, CURSOR_DY, Placement, SELECTION_OVERLAY, SELECTION_OVERLAY_PLACEMENT,
};

/// Self-buffer signal: re-anchor the toolbar to the cursor.
struct Reanchor;
y5_buffer!(REANCHOR_BUF: Reanchor);

/// Re-anchors the world-space selection toolbar under the cursor on every
/// selection-change *event*. Lives in the spatial world alongside `SelectSystem`
/// (which announces `SELECTION_CHANGED`), the pointer (`POINTER`), and the
/// surface registry (`SURFACE_MUT`) — so it can react to the event and apply the
/// move without touching the render path. The toolbar's create/destroy lifecycle
/// stays on the render path (it needs the renderer); this system only moves an
/// already-shown instance.
#[derive(Default)]
pub struct SelectionOverlaySystem;

impl System for SelectionOverlaySystem {
    fn name(&self) -> &'static str {
        "select_overlay"
    }

    fn register(&mut self, builder: &mut WorldBuilder) {
        builder.receive(&SELECTION_CHANGED, Self::on_selection_changed);
    }

    fn buffer(&mut self, cx: &mut BufferCx, _message: Box<dyn Any>) {
        // Screen placement is fixed; only the world placement follows the cursor.
        if SELECTION_OVERLAY_PLACEMENT != Placement::WorldAtCursor {
            return;
        }
        // The instance is created on the render path; nothing to move until then.
        let Some(handle) = cx.kernel.get(&SELECTION_OVERLAY).handle else {
            return;
        };
        let Some(screen) = cx.kernel.try_get(&SCREEN) else {
            return;
        };
        let scale = screen.scale;
        let m = cx.storage.get(&POINTER).motion; // y5-world logical
        // Centered horizontally on the cursor, just below it. World iced stores
        // location in logical×scale units (matches placeholder surfaces).
        let loc: Point<i32, Physical> = Point::from((
            (m.x * scale - (BAR_W as f64) / 2.0).round() as i32,
            (m.y * scale + CURSOR_DY).round() as i32,
        ));
        // SURFACE_MUT is the surface system's slot; we hold its write token and
        // run in the same world, so moving the registry instance here is safe
        // (single-threaded dispatch; the render pass reads it later this frame).
        if let Some(reg) = cx.storage.get_mut(&SURFACE_MUT).registry.as_mut() {
            reg.set_location_by_id(handle, loc);
        }
    }
}

impl SelectionOverlaySystem {
    /// Any selection change (including a primary-only change): queue a re-anchor,
    /// applied in `buffer` where storage is mutable.
    fn on_selection_changed(&mut self, cx: &mut SystemCx, _event: &SelectionChanged) {
        cx.write(&REANCHOR_BUF, Reanchor);
    }
}
