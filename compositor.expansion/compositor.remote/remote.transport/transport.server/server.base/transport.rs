use smithay::reexports::calloop;
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::calloop::channel::Sender;
use std::fs::remove_file;
use tokio::net::UnixListener;
use tokio::sync::broadcast::Receiver;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::Server;
use compositor_orchestration_core_state_base::Loop;

pub struct Transport {
    pub broadcast_transmit:
        tokio::sync::broadcast::Sender<compositor_remote_message_server_base::message::Message>,
}

pub fn create(loop_handle: LoopHandle<Loop>) -> Transport {
    // Channel for gRPC -> Calloop
    let (calloop_tx, calloop_rx) =
        calloop::channel::channel::<compositor_remote_message_client_base::message::Message>();

    // Seems that broadcast is never handled

    // Channel for Calloop -> gRPC (Capacity of 100 messages)
    let (broadcast_tx, broadcast_rx) = tokio::sync::broadcast::channel::<
        compositor_remote_message_server_base::message::Message,
    >(100);

    // Insert the Calloop receiver
    // This implements incoming_buffer populating in loop
    loop_handle
        .insert_source(calloop_rx, |event, _meta, state: &mut Loop| {
            if let calloop::channel::Event::Msg(cmd) = event {
                state.inner.kernel.get_mut(&compositor_orchestration_driver_remote_base::base::RPC_MUT).incoming_buffer.push(cmd);
            }
        })
        .unwrap();

    // Start background gRPC server, passing the channels
    background(calloop_tx);

    return Transport {
        broadcast_transmit: broadcast_tx,
    };
}

// Receiver
fn background(calloop_tx: Sender<compositor_remote_message_client_base::message::Message>) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async move {
            let socket_path = "/tmp/y5-compositor-rpc.sock";

            // Crucial: Clean up the old socket file if the compositor crashed previously
            let _ = remove_file(socket_path);

            let uds = UnixListener::bind(socket_path).unwrap();
            let uds_stream = UnixListenerStream::new(uds);

            // Initialize a new message receiver from the generated binding
            let service =
                compositor_remote_message_client_base::message::MessageClientReceiver { calloop_tx };

            info!("Starting gRPC server on {}", socket_path);

            Server::builder()
                // Every service must be added here or it wont be supported
                .add_service(compositor_remote_message_client_base::bind::navigator::navigator_server::NavigatorServer::new(service.clone()))
                .add_service(compositor_remote_message_client_base::bind::debug::debug_server::DebugServer::new(service.clone()))
                .add_service(compositor_remote_message_client_base::bind::selection::selection_server::SelectionServer::new(service.clone()))
                .serve_with_incoming(uds_stream)
                .await
                .unwrap();
        });
    });
}
