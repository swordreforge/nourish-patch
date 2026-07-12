use smithay::utils::{Logical, Point};

/// Per-seat touch state: tracks active touch points by slot ID.
/// Each touchscreen touch is identified by a `slot` (the touch point ID
/// assigned by the kernel/libinput, unique among currently active touches).
/// The bus receives a `Touch` event for each slot at each phase transition.
#[derive(Debug, Default)]
pub struct TouchState {
    /// Currently active touch points: (slot, position in world storage space).
    pub points: Vec<(i32, Point<f64, Logical>)>,
}

impl TouchState {
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    /// Record a touch down at the given position.
    pub fn down(&mut self, slot: i32, pos: Point<f64, Logical>) {
        self.points.push((slot, pos));
    }

    /// Update an active touch point's position.
    /// No-op if the slot is not tracked.
    pub fn motion(&mut self, slot: i32, pos: Point<f64, Logical>) {
        if let Some(entry) = self.points.iter_mut().find(|(s, _)| *s == slot) {
            entry.1 = pos;
        }
    }

    /// Remove an active touch point.
    /// No-op if the slot is not tracked.
    pub fn up(&mut self, slot: i32) {
        self.points.retain(|(s, _)| *s != slot);
    }

    /// Remove all active touch points (e.g. on cancel).
    pub fn cancel(&mut self) {
        self.points.clear();
    }

    /// Returns true if any touch point is active.
    pub fn is_active(&self) -> bool {
        !self.points.is_empty()
    }

    /// The number of currently active touch points.
    pub fn active_count(&self) -> usize {
        self.points.len()
    }
}
