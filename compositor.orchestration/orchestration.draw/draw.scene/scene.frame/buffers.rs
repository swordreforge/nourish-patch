use smithay::utils::{Physical, Size};
use compositor_orchestration_core_state_base::Loop;

pub fn update<R>(state: &mut Loop, renderer: &mut R, size: Size<i32, Physical>) {
    // CHECK: Right now the incoming buffer tied here is bad- render is not guaranteed unless the GRPC method forces a repaint
    // Also there are much better ways to handle the RPC events, so that they respond almost immediately, not waiting for a frame.
    let rpc_incoming: Vec<_> = state.inner.kernel.get_mut(&compositor_orchestration_driver_remote_base::base::RPC_MUT).incoming_buffer.drain(..).collect();
    for message in rpc_incoming {
        compositor_remote_client_handle_base::handle::execute(state, message);
    }
}
