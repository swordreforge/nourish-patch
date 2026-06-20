use smithay::wayland::xdg_activation::XdgActivationTokenData;
use std::sync::Arc;
use compositor_orchestration_core_state_base::Loop;
use compositor_y5_placeholder_protocol_base::message::{PlaceholderAction, PlaceholderMessage};
use compositor_y5_placeholder_record_base::placeholder::PlaceholderLaunchToken;

pub fn delegate(
    _loop: &mut Loop,
    message: compositor_y5_placeholder_protocol_base::message::PlaceholderMessage,
) {
    match message.action {
        PlaceholderAction::Save(newplan) => {
            // Get the handle and dispatch the message
            let record = _loop.inner.placeholder_mut()
                .modify_visible(&message.uuid, move |placeholder| {
                    placeholder.launch = newplan
                });

            if record.is_none() {
                return;
            }

            let (record, handle) = {
                let (record, handle) = record.unwrap();
                (record.clone(), handle.clone())
            };
            // The user edited a launcher's plan → persist the edit IMMEDIATELY.
            compositor_support_system_persist_mark_base::base::mark_world(_loop.inner.worlds.active_id(), true);

            if let Some(registry) = &mut _loop.inner.surface_mut().registry {
                let handler_instance = registry.instance_mut(handle).unwrap();
                handler_instance.runtime_mut().queue_message(
                    compositor_y5_placeholder_surface_base::PlaceholderMessage::UpdatePlan(
                        Box::new(record.launch.clone()),
                    ),
                )
            }
        }
        PlaceholderAction::Erase() => {
            if let Some((_, handle)) = _loop.inner.placeholder_mut().erase_visible(&message.uuid) {
                // Capture the draw-order id before `destroy` consumes the handle.
                let drawable_id = uuid::Uuid::from_u128(handle.id.0 as u128);
                if let Some(ref mut registry) = _loop.inner.surface_mut().registry {
                    registry.destroy(handle);
                }
                // DrawOrder GC: the placeholder surface is world-space (registered).
                _loop.inner.remove_drawable(drawable_id);
                // Dismissing a launcher tile is a deletion → persist IMMEDIATELY.
                compositor_support_system_persist_mark_base::base::mark_world(_loop.inner.worlds.active_id(), true);
            }
        }
        PlaceholderAction::Launch() => {
            let synt = &_loop.inner.placeholder_mut().synthesizer_registry.clone();

            let mut was_launch = false;
            // Get the handle and dispatch the message
            let record =
                _loop.inner.placeholder_mut()
                    .modify_visible(&message.uuid, move |placeholder| {
                        was_launch = placeholder.launching;
                        placeholder.launching = true
                    });

            if was_launch {
                info!("ERR: Launch called while launching");
                return;
            }

            if record.is_none() {
                return;
            }

            // Push pending restoration
            let mut pending_restoration: Option<PlaceholderLaunchToken> = None;

            let (record, handle) = {
                let (record, handle) = record.unwrap();
                (record.clone(), handle.clone())
            };

            if let Some(registry) = &mut _loop.inner.surface_mut().registry {
                let app_id: Option<String> =
                    record.launch.application_data.meta.meta.app_id.clone();

                let token_data = XdgActivationTokenData {
                    app_id,
                    ..XdgActivationTokenData::default()
                };
                let (token, _) = _loop
                    .state
                    .xdg_activation
                    .xdg_activation
                    .create_external_token(token_data);

                let token_str = String::from(token.as_str());
                let extra_env: &[(String, String)] = &[
                    ("XDG_ACTIVATION_TOKEN".to_string(), token_str.clone()),
                    ("DESKTOP_STARTUP_ID".to_string(), token_str.clone()),
                ];

                let child = record.launch.execute_with_env(synt, extra_env);
                if let Ok(child) = child {
                    pending_restoration = Some(PlaceholderLaunchToken {
                        token: token_str,
                        child: child,
                    });
                }
            }

            // Write the restoration token back through the owning slot (the
            // record above is a clone; the registry borrow pinned the slot).
            if pending_restoration.is_some() {
                let mut token = pending_restoration;
                _loop.inner.placeholder_mut().modify_visible(&message.uuid, move |placeholder| {
                    if let Some(token) = token.take() {
                        placeholder.restoration = Some(token);
                    }
                });
            }

            // run restoration logic
            info!("Launch!");
        }
    }
}
