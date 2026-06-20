use compositor_support_action_canvas_layout_base::layout::{LayoutFlags, MinSize, Rect, layout, rect_eq};
use std::collections::HashMap;
use std::hash::Hash;

// ---------------------------------------------------------------
// layout_ordered: invoke `layout` multiple times in sequence,
// passing each step's output rects forward as the next step's input.
//
// Each entry in `flag_steps` is one full pass through `layout`.
// Primary, min_size, and the original windows are reused for every
// pass. The final returned map contains only windows whose rect
// changed between the very first input and the very last output.
//
// Order: flag_steps[0] runs first, flag_steps[N-1] runs last.
//
// Use this when a single flag set can't express the operation —
// e.g. center first, then dual-edge stretch (so the stretch sees
// the post-center bbox).
// ---------------------------------------------------------------
pub fn layout_ordered<W>(
    windows: Vec<(W, Rect)>,
    primary: Option<(W, Rect)>,
    flag_steps: &[LayoutFlags],
    min_size: MinSize,
) -> HashMap<W, Rect>
where
    W: Eq + Hash + Clone,
{
    if windows.is_empty() || flag_steps.is_empty() {
        return HashMap::new();
    }

    // Snapshot original input rects for the final diff.
    let original: HashMap<W, Rect> = windows.iter().map(|(w, r)| (w.clone(), *r)).collect();

    // Working state, updated between steps.
    let mut current: Vec<(W, Rect)> = windows;

    for flags in flag_steps {
        // Each step gets a fresh clone of `current` as input. The result
        // contains only changed rects for that step; merge them back into
        // current to feed the next step.
        let step_in: Vec<(W, Rect)> = current.iter().map(|(w, r)| (w.clone(), *r)).collect();

        let step_out = layout(step_in, primary.clone(), *flags, min_size);

        for (w, r) in current.iter_mut() {
            if let Some(new_rect) = step_out.get(w) {
                *r = *new_rect;
            }
        }
    }

    // Final lazy diff: only rects that ended different from the original.
    let mut out = HashMap::new();
    for (w, final_rect) in current.into_iter() {
        if let Some(orig) = original.get(&w) {
            if !rect_eq(orig, &final_rect) {
                out.insert(w, final_rect);
            }
        }
    }
    out
}
