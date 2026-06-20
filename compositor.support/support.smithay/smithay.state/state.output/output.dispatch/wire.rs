//
// Wl Output & Xdg Output
//

/// Required to handle `wl_output` (monitor) events. Even if empty, it allows Smithay's
/// internal code to process screen hot-plugs and geometry updates.

// Due to being empty, this file is no-op. impl written on delegate.
pub mod OutputHandler {}
