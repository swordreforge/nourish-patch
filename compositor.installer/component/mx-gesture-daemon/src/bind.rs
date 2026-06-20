// reexport the generated bindings
pub mod bind {
    pub mod navigator {
        tonic::include_proto!("y5.compositor.rpc.protocol.client.navigator");
    }
}