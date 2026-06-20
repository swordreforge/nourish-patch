#[macro_use]
extern crate compositor_developer_debug_instance_record;

pub mod bootstrap;
pub mod driver;
pub mod entry;
pub mod frame;
pub mod handlers;
pub mod input;
pub mod state;
pub mod driver_shell_notifier;
pub mod grpc;
pub mod driver_loop;
pub mod wayland_loop;
pub mod broadcast_loop;

pub use entry::spawn_overlay_thread;
pub use state::{OverlayClient, WindowInfo};