pub mod message;

// reexport the generated bindings
pub mod bind {
    pub mod canvas {
        tonic::include_proto!("y5.compositor.rpc.protocol.broadcast.canvas");
    }
}

// pub mod message;
//
// // pub use bind::*;
//
// pub use message::*;