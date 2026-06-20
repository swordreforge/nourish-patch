use compositor_orchestration_core_state_base::Loop;
use compositor_support_action_camera_find_base::find::{Angle, Direction, Snap};

/// Number of fingers that triggers directional navigation.
const FINGERS: u32 = 3;

/// Minimum accumulated swipe distance (libinput logical units) before a swipe
/// counts as an intentional directional flick rather than incidental motion.
const SWIPE_MAGNITUDE_THRESHOLD: f64 = 100.0;

/// Snap granularity for the swipe angle — matches the remote/gRPC view path
/// (`compositor.remote` builds `Snap::Sixteenth`) so all input sources agree.
const SNAP: Snap = Snap::Sixteenth;

/// Called from the rim on `GestureSwipeEnd`. Reads the seat's swipe accumulator
/// (latched on the Orchestrator across Begin→Update*→End) and, on an intentional
/// 3-finger swipe, drives the focused world's directional view toward the swipe
/// angle. One-shot: the navigator eases the rest.
///
/// Angle convention matches `find.angle` / the MX-gesture daemon: degrees in
/// [0, 360), 0° = Right, 90° = Down (clockwise). libinput `delta_y` is
/// screen-down-positive, so `atan2(acc_y, acc_x)` needs no sign flip.
pub fn swipe_end(loop_: &mut Loop, cancelled: bool) {
    let (fingers, ax, ay) = {
        let acc = &loop_.inner.gesture;
        (acc.fingers, acc.acc_x, acc.acc_y)
    };

    if cancelled || fingers != FINGERS {
        return;
    }

    let magnitude = (ax * ax + ay * ay).sqrt();
    if magnitude < SWIPE_MAGNITUDE_THRESHOLD {
        return;
    }

    let angle = ay.atan2(ax).to_degrees().rem_euclid(360.0);
    trace!("3-finger swipe: angle={angle:.1} magnitude={magnitude:.1}");

    compositor_y5_navigator_interface_base::interface::move_direction(
        loop_,
        Direction::Diagonal(Angle(angle), SNAP),
        false,
    );
}
