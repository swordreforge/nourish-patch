use std::time::Instant;
use compositor_support_smithay_state_fractional_config::{DebounceCycle, FractionalScaleConfig};

/// Snap a raw zoom value into the configured scale lattice.
pub fn snap(cfg: &FractionalScaleConfig, zoom: f64) -> f64 {
    let clamped = zoom.clamp(cfg.min_scale, cfg.max_scale);
    let step = cfg.step.max(f64::EPSILON);
    (clamped / step).round() * step
}

/// Whether enough time has passed since the last emit.
pub fn rate_limit_clear(cfg: &FractionalScaleConfig, last_emit_at: Option<Instant>, now: Instant) -> bool {
    match last_emit_at {
        None => true,
        Some(t) => now.duration_since(t) >= cfg.min_interval,
    }
}

/// Result of a single debounce tick.
pub struct TickResult {
    pub last_observed_target: Option<f64>,
    pub cycle: Option<DebounceCycle>,
    pub pending_emit: Option<f64>,
    pub last_emitted_scale: Option<f64>,
    pub last_emit_at: Option<Instant>,
    pub emit: Option<f64>,
}

pub fn run_tick(
    cfg: &FractionalScaleConfig,
    last_observed_target: Option<f64>,
    cycle: Option<DebounceCycle>,
    pending_emit: Option<f64>,
    last_emitted_scale: Option<f64>,
    last_emit_at: Option<Instant>,
    raw_zoom: f64,
) -> TickResult {
    let zoom = raw_zoom + cfg.auto_increment;
    let now = Instant::now();
    let target = snap(cfg, zoom);

    let changed = match last_observed_target {
        None => false,
        Some(prev) => (prev - target).abs() >= cfg.step * 0.5,
    };
    let last_observed_target = Some(target);

    let mut cycle = cycle;
    let mut pending_emit = pending_emit;

    if changed {
        match &mut cycle {
            Some(c) => { c.quiet_after = now + cfg.debounce_quiet; }
            None => {
                cycle = Some(DebounceCycle {
                    started_at: now,
                    quiet_after: now + cfg.debounce_quiet,
                });
            }
        }
        if pending_emit.is_some() {
            pending_emit = Some(target);
        }
    }

    if let Some(c) = cycle {
        let quiet_fired = now >= c.quiet_after;
        let max_fired = now.duration_since(c.started_at) >= cfg.debounce_max;
        if quiet_fired || max_fired {
            if last_emitted_scale != Some(target) {
                pending_emit = Some(target);
            }
            cycle = None;
        }
    }

    let mut emit = None;
    let mut last_emitted_scale = last_emitted_scale;
    let mut last_emit_at = last_emit_at;
    let mut pending_emit = pending_emit;

    if let Some(scale) = pending_emit {
        if rate_limit_clear(cfg, last_emit_at, now) {
            last_emitted_scale = Some(scale);
            last_emit_at = Some(now);
            pending_emit = None;
            emit = Some(scale);
        }
    }

    TickResult { last_observed_target, cycle, pending_emit, last_emitted_scale, last_emit_at, emit }
}
