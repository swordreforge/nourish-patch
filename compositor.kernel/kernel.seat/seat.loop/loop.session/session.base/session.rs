//! The session notifier as a polled loop source (Law 4: generic over the loop
//! user-data type; registration and `Loop`-touching closures live in
//! `native.wire/wire.session`).

pub use smithay::backend::session::libseat::LibSeatSessionNotifier as SessionSource;
pub use smithay::backend::session::Event as SessionEvent;

/// Identity adapter, named so the wiring reads as polling a seat loop source.
pub fn source(notifier: SessionSource) -> SessionSource {
    notifier
}
