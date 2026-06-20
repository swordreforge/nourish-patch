use compositor_remote_message_server_base::message::Message;
use crate::driver_loop::{Pinger, Tick};

pub fn spawn(
    mut receiver: tokio::sync::broadcast::Receiver<Message>,
    pinger: Pinger,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap_or_else(|e| abort!("bridge runtime: {e:?}"));

        rt.block_on(async move {
            loop {
                match receiver.recv().await {
                    Ok(msg) => pinger.push(Tick::Message(msg)),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("broadcast lagged skipped={n}");
                    }
                    Err(_) => break,
                }
            }
        });
    })
}