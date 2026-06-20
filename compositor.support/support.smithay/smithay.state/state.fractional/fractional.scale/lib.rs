use std::time::Instant;
use smithay::wayland::fractional_scale::FractionalScaleManagerState;
pub use compositor_support_smithay_state_fractional_config::{DebounceCycle, FractionalScaleConfig};
use compositor_support_smithay_state_fractional_debounce::run_tick;

pub struct Fractional {
    pub state: FractionalScaleManagerState,
    pub cfg: FractionalScaleConfig,
    pub last_observed_target: Option<f64>,
    pub cycle: Option<DebounceCycle>,
    pub pending_emit: Option<f64>,
    pub last_emitted_scale: Option<f64>,
    pub last_emit_at: Option<Instant>,
}

impl Fractional {
    pub fn set_config(&mut self, cfg: FractionalScaleConfig) {
        self.cfg = cfg;
    }

    pub fn tick(&mut self, zoom: f64) -> Option<f64> {
        let r = run_tick(
            &self.cfg,
            self.last_observed_target,
            self.cycle,
            self.pending_emit,
            self.last_emitted_scale,
            self.last_emit_at,
            zoom,
        );
        self.last_observed_target = r.last_observed_target;
        self.cycle = r.cycle;
        self.pending_emit = r.pending_emit;
        self.last_emitted_scale = r.last_emitted_scale;
        self.last_emit_at = r.last_emit_at;
        r.emit
    }

    pub fn last_emitted(&self) -> Option<f64> {
        self.last_emitted_scale
    }
}
