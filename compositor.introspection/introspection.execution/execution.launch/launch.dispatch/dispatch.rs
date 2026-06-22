//! `LaunchWorker`: the handle held in compositor state for off-thread launches.

use std::sync::mpsc;
use std::thread;

use smithay::reexports::calloop::channel::Sender as CalloopSender;

use compositor_introspection_execution_launch_execute::execute::execute;
use compositor_introspection_execution_launch_types::types::{LaunchOutcome, LaunchRequest};

/// Submit launches off the calloop thread. Cloneable; the worker thread lives
/// for the process lifetime.
#[derive(Clone)]
pub struct LaunchWorker {
    tx: mpsc::Sender<LaunchRequest>,
}

impl LaunchWorker {
    /// Spawn the worker thread. `outcomes` is the calloop side; the caller must
    /// insert its receiver as a loop source and dispatch each outcome.
    pub fn spawn(outcomes: CalloopSender<LaunchOutcome>) -> Self {
        let (tx, rx) = mpsc::channel::<LaunchRequest>();
        thread::Builder::new()
            .name("y5-launch".into())
            .spawn(move || run(rx, outcomes))
            .unwrap_or_else(|e| abort!("spawn launch worker: {e:?}"));
        Self { tx }
    }

    /// Queue a launch. Dropped silently if the worker thread is gone.
    pub fn submit(&self, req: LaunchRequest) {
        let _ = self.tx.send(req);
    }
}

fn run(rx: mpsc::Receiver<LaunchRequest>, outcomes: CalloopSender<LaunchOutcome>) {
    while let Ok(req) = rx.recv() {
        let outcome = execute(&req);
        if outcomes.send(outcome).is_err() {
            break; // calloop receiver dropped → the loop is shutting down.
        }
    }
}
