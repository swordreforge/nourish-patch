use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use smithay_client_toolkit::shell::{
    wlr_layer::{Anchor, KeyboardInteractivity, Layer},
    WaylandSurface,
};
use tokio::sync::broadcast::Receiver;
use compositor_monitor_devtool_scene_base::app::CompositorSnapshot;
use compositor_remote_message_server_base::message::Message;
use crate::bootstrap::{bootstrap, Bootstrapped};
use crate::driver::IcedDriver;
use crate::driver_loop::{Buffer, PingChannel, Pinger, Tick};
use crate::{broadcast_loop, handlers, wayland_loop};

pub fn spawn_overlay_thread(receiver: Receiver<compositor_remote_message_server_base::message::Message>) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("overlay".into())
        .spawn(|| {
            if let Err(e) = run(receiver) {
                error!("overlay thread crashed: {e:?}");
                std::process::abort();
            }
        })
        .unwrap_or_else(|e| abort!("spawn overlay thread: {e:?}"))
}



fn run(receiver: Receiver<Message>) -> Result<(), Box<dyn std::error::Error>> {
    // sleep(Duration::from_secs(5));
    let Bootstrapped { conn, mut event_queue, mut state, snapshot } = bootstrap()?;
    let qh = event_queue.handle();

    // ── Phase 2a: create the wl_surface. ─────────────────────────────────
    let wl_surface = state.compositor_state.create_surface(&qh);

    // ── Phase 2b: send metadata for this surface BEFORE the layer surface
    //             is created. The compositor will associate the payload
    //             with the surface, and when the layer-shell handler fires,
    //             it can look up our metadata immediately.
    let payload = build_overlay_metadata_payload(&snapshot);
    // state.custom_proto.set_overlay_metadata(&wl_surface, payload);

    // ── Phase 2c: create the layer surface from the wl_surface. ──────────
    let layer = state.layer_shell.create_layer_surface(
        &qh,
        wl_surface,
        Layer::Overlay,
        Some("overlay"),
        None,
    );


    // Anchor only to bottom — surface centers horizontally automatically.
    layer.set_anchor(Anchor::BOTTOM);

    // 12px gap from the bottom of the screen.
    layer.set_margin(0, 0, 100, 0);  // top, right, bottom, left

    // Don't reserve space — let windows render under us.
    layer.set_exclusive_zone(0);

    // Ask for a specific size. The compositor's configure may override this.
    layer.set_size(440, 120);
    state.requested_size = (440, 120);

    // layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
    layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
    // layer.set_size(800, 600);

    layer.commit(); // first commit, no buffer → triggers configure
    state.layer = Some(layer);

    let buffer: Buffer = Arc::new(Mutex::new(Vec::new()));
    let ping_channel = PingChannel::new();
    let pinger = Pinger {
        sender: ping_channel.sender.clone(),
        buffer: buffer.clone(),
    };

    let _wayland_handle = wayland_loop::spawn(conn.clone(), pinger.clone());
    let _bridge_handle = broadcast_loop::spawn(receiver, pinger.clone());



    // ── Phase 3: main loop. ──────────────────────────────────────────────
    while !state.should_exit {
        // Take the buffer atomically.
        let ticks = {
            // Take buffer
            let mut buf = buffer.lock().unwrap();
            let taken = std::mem::take(&mut *buf);
            // Flush ping
            while let Ok(()) = ping_channel.receiver.try_recv() {}
            taken
            // lock released
        };

        if ticks.is_empty() {
            let _ = ping_channel.receiver.recv();
            continue;
        }

        // Process each tick.
        let mut had_wayland = false;
        for tick in ticks {
            match tick {
                Tick::Wayland => had_wayland = true,
                Tick::Message(msg) => {
                    state.broadcast_dispatch(msg);
                }
            }
        }

        // Dispatch Wayland once if any wayland ticks accumulated.
        if had_wayland {
            event_queue.dispatch_pending(&mut state)?;
        }

        crate::frame::pump(&mut state, &qh);
    }

    Ok(())
}


/// Build the payload string sent via set_overlay_metadata.
/// Right now this is a placeholder — fill in whatever your compositor expects
/// (likely JSON: overlay role, ID, layer config, etc.)
fn build_overlay_metadata_payload(_snapshot: &CompositorSnapshot) -> String {
    // Example: r#"{"role":"launcher","version":1}"#.into()
    String::new()
}