use std::time::Instant;

#[derive(Clone)]
pub struct Travel {
    pub position: Option<Target<(f64, f64)>>,
    pub zoom: Option<Target<f64>>,

    pub duration: Option<f64>,
    // The time the machine has received the first update call
    pub time_start: Option<Instant>,
    // The current time which is incremented by delta_time every frame.
    // If for whatever reason the machine halts for 2 seconds, it will know that it has halted(reflected in delta_time)
    // If for whatever reason normalized 'elapsed' is needed, then 'Pause' it explicitly.
    // pub time_current: Option<Instant>,
}

#[derive(Clone)]
pub struct Target<R> {
    pub start: Option<R>,
    pub target: R,
}
