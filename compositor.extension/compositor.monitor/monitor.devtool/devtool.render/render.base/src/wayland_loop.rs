use wayland_client::Connection;
use crate::driver_loop::{Pinger, Tick};

pub fn spawn(conn: Connection, pinger: Pinger) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            // Flush outgoing requests first.
            if conn.flush().is_err() {
                break;
            }

            // Block until Wayland has data to read.
            match conn.prepare_read() {
                Some(guard) => {
                    if guard.read().is_err() {
                        break;
                    }
                    pinger.push(Tick::Wayland);
                }
                None => {
                    // Events buffered but not yet dispatched. Tell the main thread,
                    // then yield briefly so we don't spin.
                    // pinger.push(Tick::Wayland);
                    // std::thread::yield_now();
                    // Or: std::thread::sleep(Duration::from_micros(100));
                }
            }

            // Tell the main loop there's something to dispatch.
            // pinger.push(Tick::Wayland);
        }
    })
}