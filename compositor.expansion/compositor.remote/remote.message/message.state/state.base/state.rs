pub struct State {
    pub incoming_buffer: Vec<compositor_remote_message_client_base::message::Message>,
}

impl State {
    pub fn new() -> Self {
        return Self {
            incoming_buffer: vec![],
        };
    }
}
