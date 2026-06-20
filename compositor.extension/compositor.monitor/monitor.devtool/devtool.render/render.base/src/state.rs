use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    seat::SeatState,
    shell::wlr_layer::{LayerShell, LayerSurface},
};
use wayland_client::protocol::{wl_keyboard, wl_pointer};
use compositor_monitor_devtool_scene_base::selection::SelectionAction;
use compositor_monitor_devtool_scene_base::ui::Message;
use compositor_monitor_server_protocol_base::protocol::y5_proto::y5_compositor_unstable_client_v1::y5_compositor_manager_v1::Y5CompositorManagerV1;
use compositor_remote_message_client_base::bind::selection;
use compositor_remote_message_client_base::bind::selection::{action, align, distribute, stack};
use crate::driver::{IcedDriver, MessageHandler};
use crate::grpc::GrpcClient;

pub struct OverlayClient {
    pub redraw_requested: Arc<AtomicBool>,
    pub layout_invalidated: Arc<AtomicBool>,

    pub grpc: GrpcClient,
    // sctk machinery
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub layer_shell: LayerShell,

    // Custom protocol
    // pub custom_proto: Y5CompositorManagerV1,

    // Layer surface — created after metadata is sent.
    pub layer: Option<LayerSurface>,
    pub configured_size: Option<(u32, u32)>,

    // Iced runtime — created lazily on first configure.
    pub iced: Option<IcedDriver>,
    pub drawn_initial_frame: bool,
    pub _tokio_runtime: tokio::runtime::Runtime,

    // Input
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub pointer_position: (f64, f64),

    pub requested_size: (u32, u32),

    // Frame pacing
    pub frame_in_flight: bool,
    pub frame_callback_fired: bool,

    // Lifecycle
    pub should_exit: bool,
}

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: u64,
    pub title: String,
    pub app_id: String,
}

impl OverlayClient {}

pub struct OverlayMessageHandler {
    pub grpc: GrpcClient,
}

impl MessageHandler for OverlayMessageHandler {
    fn handle(&mut self, message: &Message) {
        match message {
            Message::SelectNotify(_) => {}
            Message::ButtonPressed(n) => {
                match n {
                    1 => {
                        self.grpc.zoom_reset();
                    }

                    2 => {
                        self.grpc.view_directional();
                    }
                    &_ => {
                        self.grpc.debug_numeric(n.clone() as u32)
                        // self.grpc.view_directional();
                    }
                }
            }
            Message::ExecuteSelection(actions, alternative) => {

                // : Vec<selection::action::Action>
                let actions: Vec<selection::Action> = actions
                    .iter()
                    .map(|a| {
                        let action: selection::action::Action = match a {
                            SelectionAction::ScaleToFit(_) => {
                              abort!("use scale_to_fit");
                            },
                            SelectionAction::AlignTop => {
                                selection::action::Action::Align(
                                    selection::Align {
                                        action: Some(
                                            selection::align::Action::Top(selection::align::Modifier {
                                                stretch: *alternative,
                                            })
                                        ),
                                    }
                                )
                            }
                            SelectionAction::AlignBottom => {
                                selection::action::Action::Align(
                                    selection::Align {
                                        action: Some(
                                            selection::align::Action::Bottom(selection::align::Modifier {
                                                stretch: *alternative,
                                            })
                                        ),
                                    }
                                )
                            }
                            SelectionAction::AlignLeft => {
                                selection::action::Action::Align(
                                    selection::Align {
                                        action: Some(
                                            selection::align::Action::Left(selection::align::Modifier {
                                                stretch: *alternative,
                                            })
                                        ),
                                    }
                                )
                            }
                            SelectionAction::AlignVerticalCenter => {
                                selection::action::Action::Align(
                                    selection::Align {
                                        action: Some(
                                            selection::align::Action::CenterVertical(selection::align::Modifier {
                                                stretch: *alternative,
                                            })
                                        ),
                                    }
                                )
                            }
                            SelectionAction::AlignHorizontalCenter => {
                                selection::action::Action::Align(
                                    selection::Align {
                                        action: Some(
                                            selection::align::Action::CenterHorizontal(selection::align::Modifier {
                                                stretch: *alternative,
                                            })
                                        ),
                                    }
                                )
                            }
                            SelectionAction::AlignRight => {
                                selection::action::Action::Align(
                                    selection::Align {
                                        action: Some(
                                            selection::align::Action::Right(selection::align::Modifier {
                                                stretch: *alternative,
                                            })
                                        ),
                                    }
                                )
                            }
                            SelectionAction::DistributeHorizontal => {
                                selection::action::Action::Distribute(
                                    selection::Distribute {
                                        action: Some(
                                            selection::distribute::Action::Horizontal(selection::distribute::Modifier {
                                                start: *alternative
                                            })
                                        ),
                                    }
                                )
                            }
                            SelectionAction::DistributeVertical => {
                                selection::action::Action::Distribute(
                                    selection::Distribute {
                                        action: Some(
                                            selection::distribute::Action::Vertical(selection::distribute::Modifier {
                                                start: *alternative
                                            })
                                        ),
                                    }
                                )
                            }
                            SelectionAction::StackHorizontal => {
                                selection::action::Action::Stack(
                                    selection::Stack {
                                        action: Some(
                                            selection::stack::Action::Horizontal(true)
                                        ),
                                    }
                                )
                            }
                            SelectionAction::StackVertical => {
                                selection::action::Action::Stack(
                                    selection::Stack {
                                        action: Some(
                                            selection::stack::Action::Vertical(true)
                                        ),
                                    }
                                )
                            }
                        };


                        selection::Action {
                            action: Some(
                                action
                            )
                        }
                    })
                    .collect::<Vec<selection::Action>>();

                info!("ExecuteSelection {:?}", actions);
                self.grpc.selection_layout(actions)
            }
            Message::ExecuteScaleToFit(actions) => {
                self.grpc.scale_to_fit(selection::FitAspect{
                    perceived: actions.perceived,
                    max: actions.max,
                    horizontal: actions.horizontal,
                    vertical: actions.vertical,
                })
            }
            _ => {}
        }
    }
}
