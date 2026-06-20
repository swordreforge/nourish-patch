use crate::bind;
use tokio::sync::oneshot;
use tonic::{IntoRequest, Request, Response, Status};
use crate::bind::canvas::selection::Notify;

#[derive(Clone)]
pub struct Message {
    pub Value: ServerEvent,
}

// Unify to a single message type.
compositor_remote_message_macro_base::define_broadcasts! {
    master: ServerEvent,
    packages: {
        Canvas {
            namespace: canvas_events,
            enum: CanvasMessage,
            messages: {
                Notify(bind::canvas::selection::Notify);
            }
        },
    }
}