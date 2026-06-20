pub use compositor_support_action_camera_fit_aspect_types::{Flags, Origin, Point, Size};
use compositor_support_action_camera_fit_aspect_types::{MIN_H, MIN_W, SNAP_THRESHOLD};
use compositor_support_action_camera_fit_aspect_extend::compute_minimal_extension;
use compositor_support_action_camera_fit_aspect_maximize::compute_maximized;

pub fn fit_aspect_ratio(
    total_size: Size,
    perceived_total_size: Size,
    element_size: Size,
    element_origin: Origin,
    element_position: Point,
    flags: Flags,
) -> (Size, Point) {
    let center = match element_origin {
        Origin::Center => element_position,
        Origin::TopLeft => Point {
            x: element_position.x + element_size.w * 0.5,
            y: element_position.y + element_size.h * 0.5,
        },
    };

    let new_size = compute_new_size(total_size, perceived_total_size, element_size, flags);

    let new_position = match element_origin {
        Origin::Center => center,
        Origin::TopLeft => Point {
            x: center.x - new_size.w * 0.5,
            y: center.y - new_size.h * 0.5,
        },
    };

    (new_size, new_position)
}

fn compute_new_size(total: Size, perceived: Size, elem: Size, flags: Flags) -> Size {
    let target_ratio = total.w / total.h;

    let (cap, snap_target) = if flags.scale_to_perceived {
        let cap = Size {
            w: perceived.w.min(total.w),
            h: perceived.h.min(total.h),
        };
        (cap, cap)
    } else {
        (total, total)
    };

    let max_w = cap.w.max(MIN_W);
    let max_h = cap.h.max(MIN_H);

    let start = Size {
        w: elem.w.clamp(MIN_W, max_w),
        h: elem.h.clamp(MIN_H, max_h),
    };

    let chosen = if flags.maximize {
        compute_maximized(start, target_ratio, max_w, max_h)
    } else {
        compute_minimal_extension(start, target_ratio, max_w, max_h)
    };

    if chosen.w >= snap_target.w * (1.0 - SNAP_THRESHOLD)
        || chosen.h >= snap_target.h * (1.0 - SNAP_THRESHOLD)
    {
        return Size {
            w: snap_target.w.min(total.w),
            h: snap_target.h.min(total.h),
        };
    }

    chosen
}
