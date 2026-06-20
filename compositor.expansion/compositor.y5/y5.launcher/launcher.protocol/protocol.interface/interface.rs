use smithay::backend::renderer::gles::GlesRenderer;
use smithay::reexports::rustix::net::socket;
use compositor_introspection_launchplan_plan_base::exec::{sanitise_unit_name, short_random, spawn_via_exec, spawn_via_systemd};
use compositor_orchestration_core_state_base::Loop;
use compositor_y5_launcher_protocol_message::message::{
    ExternalAction, InternalAction, LauncherMessage, Source,
};

pub fn handle(
    _loop: &mut Loop,
    renderer: &mut GlesRenderer,
    message: compositor_y5_launcher_protocol_message::message::LauncherMessage,
) {
    match message.message {
        Source::Internal(Type) => match Type {
            InternalAction::Start => {
                compositor_y5_launcher_interface_base::interface::start(_loop, renderer);
            }
        },
        Source::External(Message) => {
            match Message {
                ExternalAction::Start {
                    id,
                    bin,
                    args,
                    direction,
                } => {
                    // Immediately launch. later hook up with state.
                    // (1) Mint XDG activation token via smithay's xdg_activation
                    // let token = self.activation_state.create_external_token(/* serial */);
                    // let token_str = token.token().to_string();

                    // (2) Build unique unit name
                    let unit = format!(
                        "y5-app-{}-{}.service",
                        sanitise_unit_name(&id),
                        short_random(),
                    );

                    let socket_name = _loop.inner.loader.socket_name.clone();
                    let socket= socket_name.into_string().unwrap();

                    // (3) Env with activation token
                    let extra_env: Vec<(String, String)> = vec![
                        ("WAYLAND_DISPLAY".into(), socket), // ("XDG_ACTIVATION_TOKEN".into(), token_str.clone()),
                        ("DISPLAY".into(), String::new()),  // unset X fallback
                        ("XDG_SESSION_TYPE".into(), "wayland".into()),
                    ];

                    // (4) Spawn (returns in ms, doesn't block)
                    if let Err(e) = spawn_via_exec(&bin, args, &extra_env, &unit) {
                        warn!("launch failed: {e}");
                    }

                    let Some(handle) = _loop.inner.launcher_mut().handle else {
                        return;
                    };
                    let Some(ref mut registry) = _loop.inner.surface_mut().registry else {
                        return;
                    };

                    registry.destroy(handle);
                    _loop.inner.launcher_mut().handle = None;

                    // (5) Remember placement intent for the toplevel that arrives next
                    // self.pending_placements.insert(token_str, direction);
                }
                ExternalAction::Exit => {
                    let Some(handle) = _loop.inner.launcher_mut().handle else {
                        return;
                    };
                    let Some(ref mut registry) = _loop.inner.surface_mut().registry else {
                        return;
                    };

                    registry.destroy(handle);
                    _loop.inner.launcher_mut().handle = None;
                }
            }
        }
    }
}
