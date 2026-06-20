use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced_graphics::shell::{Notifier, Shell}; // I also tried this

pub struct OverlayNotifier {
    pub redraw: Arc<AtomicBool>,
    pub invalidate: Arc<AtomicBool>,
}

impl Notifier for OverlayNotifier {
    fn tick(&self) {
        // The renderer wants its per-frame tick. Schedule a redraw so the
        // next frame happens and Renderer::tick gets called.
        self.redraw.store(true, Ordering::Relaxed);
    }

    fn request_redraw(&self) {
        self.redraw.store(true, Ordering::Relaxed);
    }
    fn invalidate_layout(&self) {
        self.invalidate.store(true, Ordering::Relaxed);
        self.redraw.store(true, Ordering::Relaxed);
    }
}