use std::sync::{Arc, Mutex};
use std::sync::mpsc::{sync_channel, SyncSender, Receiver, TryRecvError};

pub enum Tick {
    Wayland,
    Message(compositor_remote_message_server_base::message::Message),
}

pub type Buffer = Arc<Mutex<Vec<Tick>>>;

pub struct PingChannel {
    pub sender: SyncSender<()>,
    pub receiver: Receiver<()>,
}

impl PingChannel {
    pub fn new() -> Self {
        // Cap 1: subsequent pings while one's pending are no-ops, which is exactly what we want.
        let (sender, receiver) = sync_channel(1);
        Self { sender, receiver }
    }
}

#[derive(Clone)]
pub struct Pinger {
    pub sender: SyncSender<()>,
    pub buffer: Buffer,
}

impl Pinger {
    pub fn push(&self, tick: Tick) {
        let mut buf = self.buffer.lock().unwrap();
        buf.push(tick);
        let _ = self.sender.try_send(());
        // lock released here
    }

    // pub fn ping(&self) {
    //     let _ = self.sender.try_send(());
    // }
}