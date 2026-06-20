pub struct State {
    pub incoming_buffer: Vec<compositor_remote_message_client_base::message::Message>,
    pub broadcast: tokio::sync::broadcast::Sender<compositor_remote_message_server_base::message::Message>,
}

impl State {
    pub fn new(broadcast: tokio::sync::broadcast::Sender<compositor_remote_message_server_base::message::Message>) -> Self {
        return Self {
            incoming_buffer: vec![],
            broadcast: broadcast,
        };
    }
}
