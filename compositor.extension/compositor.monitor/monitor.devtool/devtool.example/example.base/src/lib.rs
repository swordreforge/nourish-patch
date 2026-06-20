//! # compositor_monitor_devtool_example_base
//!
//! A minimal bidirectional test UI for the iced-dmabuf integration.
//!
//! Two files:
//! - [`counter_ui`] — the UI itself: a counter with Increment/Reset buttons,
//!   tick display, and a status line. Receives messages from the compositor
//!   and emits its own.
//! - [`wiring`] — a heavily-commented walkthrough of how to wire this into
//!   your compositor binary. Code that references compositor state is in
//!   comments; copy snippets into your compositor.

#[macro_use]
extern crate compositor_developer_debug_instance_record;

pub mod counter_ui;
pub mod wiring;

pub use counter_ui::{CounterUi, OutgoingMessage};
