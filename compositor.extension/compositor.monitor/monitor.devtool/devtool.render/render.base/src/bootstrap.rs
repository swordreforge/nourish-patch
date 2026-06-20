use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    seat::SeatState,
    shell::wlr_layer::LayerShell,
};
use wayland_client::{
    globals::registry_queue_init, Connection, QueueHandle,
};
use compositor_monitor_devtool_scene_base::app::CompositorSnapshot;
use compositor_monitor_server_protocol_base::protocol::y5_proto::y5_compositor_unstable_client_v1::y5_compositor_manager_v1::Y5CompositorManagerV1;
use crate::grpc::GrpcClient;
use crate::state::{OverlayClient};
use tokio::runtime;
pub struct Bootstrapped {
    pub conn: Connection,
    pub event_queue: wayland_client::EventQueue<OverlayClient>,
    pub state: OverlayClient,
    pub snapshot: CompositorSnapshot,
}

/// Connect, bind globals, roundtrip. Does not create the layer surface yet
/// and does not call set_overlay_metadata yet (we need a wl_surface first).
pub fn bootstrap() -> Result<Bootstrapped, Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, event_queue) = registry_queue_init::<OverlayClient>(&conn)?;
    let qh: QueueHandle<OverlayClient> = event_queue.handle();

    let registry_state = RegistryState::new(&globals);
    let seat_state = SeatState::new(&globals, &qh);
    let output_state = OutputState::new(&globals, &qh);
    let compositor_state =
        CompositorState::bind(&globals, &qh).unwrap_or_else(|e| abort!("wl_compositor not available: {e:?}"));
    let layer_shell =
        LayerShell::bind(&globals, &qh).unwrap_or_else(|e| abort!("wlr-layer-shell not available: {e:?}"));

    // let custom_proto = globals
    //     .bind::<Y5CompositorManagerV1, _, _>(&qh, 1..=1, ())
    //     .expect("y5_compositor_manager_v1 not available");

    let redraw_requested = Arc::new(AtomicBool::new(false));
    let layout_invalidated = Arc::new(AtomicBool::new(false));

    let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()?;


    let grpc = GrpcClient::new("/tmp/y5-compositor-rpc.sock", tokio_runtime.handle().clone());


    let state = OverlayClient {
        drawn_initial_frame: false,
        registry_state,
        seat_state,
        output_state,
        compositor_state,
        layer_shell,
        // custom_proto,
        requested_size: (0, 0),
        layer: None,
        _tokio_runtime: tokio_runtime,
        configured_size: None,
        iced: None,
        keyboard: None,
        pointer: None,
        pointer_position: (0.0, 0.0),
        frame_in_flight: false,
        frame_callback_fired: false,
        should_exit: false,
        redraw_requested,
        layout_invalidated,
        grpc,
    };

    // The snapshot is currently empty because the protocol has no events.
    // When you add server → client events, drain them here before returning.
    let snapshot = CompositorSnapshot::default();

    Ok(Bootstrapped { conn, event_queue, state, snapshot })
}