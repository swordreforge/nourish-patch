//! Sample batch/result types delivered to the main thread.

use std::mem;
use std::time::Instant;

use smithay::reexports::calloop::channel::Sender as CalloopSender;
use uuid::Uuid;
use compositor_introspection_inference_hint_base::ApplicationData;

/// One flush event delivered to the main thread. Contains all results
/// accumulated since the previous flush.
#[derive(Debug)]
pub struct SampleBatch {
    pub results: Vec<SampleResult>,
    /// When the flush fired (not when each result was sampled).
    pub flushed_at: Instant,
}

/// One sample result. Belongs to a `SampleBatch`.
#[derive(Debug)]
pub struct SampleResult {
    pub uuid: Uuid,
    /// `None` if extraction failed (process gone, /proc unreadable).
    pub data: Option<ApplicationData>,
}

/// Drain `buffer` (if non-empty) into a `SampleBatch` and send it.
/// Returns `false` if the receiver is gone (caller should shut down).
pub fn flush(
    buffer: &mut Vec<SampleResult>,
    results_tx: &CalloopSender<SampleBatch>,
    now: Instant,
) -> bool {
    if !buffer.is_empty() {
        let batch = SampleBatch {
            results: mem::take(buffer),
            flushed_at: now,
        };
        if results_tx.send(batch).is_err() {
            return false;
        }
    }
    true
}
