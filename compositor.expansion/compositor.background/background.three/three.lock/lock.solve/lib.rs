//! Spring-solver helpers for the morph animation phases.

use compositor_background_three_lock_spring::{Solver, solve};
use std::time::Duration;

pub const SPRING_STIFFNESS: f64 = 154.0;
pub const SPRING_DAMPING: f64 = 9.54;
pub const SPRING_MASS: f64 = 0.25;

pub const BAND_DURATION_MS: u64 = 1500;
pub const BAND_STAGGER_MS: u64 = 1250;

pub fn band_solver(value_start: f64, value_target: f64, duration_ms: u64) -> Solver {
    Solver {
        stiffness: SPRING_STIFFNESS,
        damping: SPRING_DAMPING,
        mass: SPRING_MASS,
        value_start,
        value_target,
        duration: Duration::from_millis(duration_ms),
    }
}

/// Solve a band's flatness given the parent-phase time. `from→to` is the
/// band's value trajectory.
pub fn solve_band(t_phase: Duration, band_start_ms: u64, from: f64, to: f64) -> f32 {
    let start = Duration::from_millis(band_start_ms);
    if t_phase < start {
        return from as f32;
    }
    let t_band = t_phase - start;
    let solver = band_solver(from, to, BAND_DURATION_MS);
    solve(&solver, t_band).unwrap_or(to) as f32
}

pub fn solve_phase_progress(t_phase: Duration, duration_ms: u64, forward: bool) -> f32 {
    // Normalize phase time to [0, spring's natural duration].
    // The spring settles in ~250ms naturally at our params; we ask the
    // spring as if 250ms have passed for every full phase.
    let SPRING_NATURAL_MS = 250.0;
    let phase_progress = (t_phase.as_secs_f64() / (duration_ms as f64 / 1000.0)).min(1.0);
    let spring_time = Duration::from_secs_f64(phase_progress * SPRING_NATURAL_MS / 1000.0);

    let (from, to) = if forward { (0.0, 1.0) } else { (1.0, 0.0) };
    let solver = Solver {
        stiffness: SPRING_STIFFNESS,
        damping: SPRING_DAMPING,
        mass: SPRING_MASS,
        value_start: from,
        value_target: to,
        duration: Duration::from_millis(SPRING_NATURAL_MS as u64),
    };
    solve(&solver, spring_time).unwrap_or(to) as f32
}
