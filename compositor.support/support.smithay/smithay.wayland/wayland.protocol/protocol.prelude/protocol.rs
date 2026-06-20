// use wayland_server::{backend::ClientId, Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New};
//
// use compositor_monitor_server_protocol_base::protocol::y5_proto;
// use crate::state::{Loop};
//
// use y5_proto::y5_compositor_unstable_v1::y5_compositor_manager_v1::{
//     Y5CompositorManagerV1, Request,
// };

// #[derive(Debug, Clone)]
// pub struct IceMetadata {
//     payload: String,
// }
// impl Dispatch<Y5CompositorManagerV1, ()> for Loop {
//     fn request(
//         _state: &mut Self,
//         _client: &Client,
//         _resource: &Y5CompositorManagerV1,
//         request: Request,
//         _data: &(),
//         _dhandle: &DisplayHandle,
//         _data_init: &mut DataInit<'_, Self>,
//     ) {
//         match request {
//             Request::SetOverlayMetadata { surface, payload } => {
//                 smithay::wayland::compositor::with_states(&surface, |states| {
//                     states
//                         .data_map
//                         .insert_if_missing_threadsafe(|| {
//                             IceMetadata { payload }
//                         });
//                 });
//             }
//             _ => {}
//         }
//     }
//
//     fn destroyed(
//         _state: &mut Self,
//         _client: ClientId,
//         _resource: &Y5CompositorManagerV1,
//         _data: &(),
//     ) {
//     }
// }

// impl GlobalDispatch<Y5CompositorManagerV1, ()> for Loop {
//     fn bind(
//         _state: &mut Self,
//         _handle: &DisplayHandle,
//         _client: &Client,
//         resource: New<Y5CompositorManagerV1>,
//         _global_data: &(),
//         data_init: &mut DataInit<'_, Self>,
//     ) {
//         data_init.init(resource, ());
//     }
// }

// NO delegate macros — you've already written the impls directly.