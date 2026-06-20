//! VBlank timestamp interpretation + refresh math. Pure value work over
//! caller-supplied inputs (Law 1: no loop types).

use smithay::output::Mode;
use smithay::wayland::presentation::Refresh;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct VblankStamp {
    pub time: Duration,
    pub sequence: u64,
}

/// Interpret the kernel-supplied (or absent) vblank timestamp, falling back
/// to the monotonic-now the caller measured (behavior carried verbatim).
pub fn interpret(time: Option<Duration>, sequence: u64, fallback_now: Duration) -> VblankStamp {
    VblankStamp {
        time: time.unwrap_or(fallback_now),
        sequence,
    }
}

/// Refresh descriptor for presentation feedback (mode.refresh is mHz).
pub fn refresh_interval(mode: &Mode) -> Refresh {
    Refresh::fixed(interval(mode))
}

/// The plain refresh interval as a Duration — what the Law-7 timing nets
/// (throttle / predict / estimate) compute against.
pub fn interval(mode: &Mode) -> Duration {
    Duration::from_secs_f64(1_000f64 / mode.refresh.max(1) as f64)
}
