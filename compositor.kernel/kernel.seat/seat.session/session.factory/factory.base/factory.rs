//! LibSeatSession + notifier construction. (Ex wire.rs `new()` step 1.)
//! Failure policy: a compositor without a session cannot run — panic.

use smithay::backend::session::libseat::{LibSeatSession, LibSeatSessionNotifier};
use smithay::backend::session::Session;

pub fn create() -> (LibSeatSession, LibSeatSessionNotifier) {
    let (session, notifier) =
        LibSeatSession::new().expect("libseat session creation failed");
    info!("libseat session created (seat: {})", session.seat());
    (session, notifier)
}
