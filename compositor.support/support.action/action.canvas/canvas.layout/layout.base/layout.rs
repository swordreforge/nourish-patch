//! Façade: re-exports all public items previously defined here.
pub use compositor_support_action_canvas_layout_rect::{Rect, EPSILON, rect_eq};
pub use compositor_support_action_canvas_layout_minsize::MinSize;
pub use compositor_support_action_canvas_layout_flags::LayoutFlags;
pub use compositor_support_action_canvas_layout_variant::{
    DistributeVariant, distribute_variant_h, distribute_variant_v,
};
pub use compositor_support_action_canvas_layout_axis::{
    Axis, axis_min, axis_len, axis_center, axis_set_min,
};
pub use compositor_support_action_canvas_layout_side::{
    AxisAlign, Side, resolve_axis_mode, apply_axis, clamp_min,
};
pub use compositor_support_action_canvas_layout_converge::{
    EdgeSel, edge_value, median_of, converge_close,
    CLOSE_MAX_ITERS, CLOSE_SNAP_PX, CLOSE_ALPHA,
};
pub use compositor_support_action_canvas_layout_close::align_close;
pub use compositor_support_action_canvas_layout_align::align;
pub use compositor_support_action_canvas_layout_spacing::{
    pick_no_primary_spacing, pick_primary_spacing,
};
pub use compositor_support_action_canvas_layout_distrib::distribute_axis;
pub use compositor_support_action_canvas_layout_primary::{
    Side2, classify_side, distribute_with_primary,
};
pub use compositor_support_action_canvas_layout_entry::layout;
