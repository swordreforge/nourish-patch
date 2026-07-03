//! Applies forwarded settings-window messages (drained from the surface pump).
//! Preferences are mutated on the live `inner.preference` object (so the change
//! takes effect immediately) and then persisted to preferences.json. Environment
//! edits write settings.json (the UI already flagged the reboot banner); output
//! modes go via the OUTPUT_MODE_REQUEST channel (apply / confirm+persist / revert).
use compositor_developer_environment_preference_base::base as pref;
use compositor_orchestration_core_state_base::Loop;
use compositor_orchestration_driver_output_base::base::{OutputModeRequest, OUTPUT_MODE_REQUEST_MUT, OUTPUTS_SNAPSHOT, OUTPUT_RECONCILE_REQUEST_MUT};
use compositor_orchestration_driver_settings_base::base::SETTINGS_MUT;
use compositor_configurator_settings_surface_message::message::{SettingsMessage, Tab};
use compositor_configurator_network_backend_base::base::{self as wifi, WifiCmd};
use compositor_configurator_bluetooth_backend_base::base::{self as bt, BtCmd};
use compositor_orchestration_driver_audio_base::base::AUDIO;
use smithay::backend::renderer::gles::GlesRenderer;

/// Currently-connected monitors' EDID keys (from the kernel snapshot) — the set the
/// live teleport map is filtered against when it is rebuilt.
fn connected_keys(state: &Loop) -> Vec<String> {
    state
        .inner
        .kernel
        .get(&OUTPUTS_SNAPSHOT)
        .displays
        .iter()
        .filter(|d| d.connected)
        .map(|d| d.edid_key.clone())
        .collect()
}

pub fn handle(state: &mut Loop, _renderer: &mut GlesRenderer, m: SettingsMessage) {
    match m {
        SettingsMessage::Cursor(v) => {
            state.inner.preference.cursor_sensitivity = v as f64;
            let _ = pref::save(&state.inner.preference);
        }
        SettingsMessage::NaturalScroll(b) => {
            state.inner.preference.input_natural_scroll = b;
            let _ = pref::save(&state.inner.preference);
        }
        SettingsMessage::Env(e) => {
            let _ = compositor_developer_environment_config_base::base::save(&e);
        }
        SettingsMessage::Apply(a) => {
            // Per-pipe mode change on the SELECTED monitor (multi-output: every output
            // is independently driven, so this is never an active-output switch).
            *state.inner.kernel.get_mut(&OUTPUT_MODE_REQUEST_MUT) = Some(OutputModeRequest::Apply {
                edid_key: a.edid_key,
                width: a.mode.width,
                height: a.mode.height,
                refresh_mhz: a.mode.refresh_mhz,
            });
        }
        SettingsMessage::Keep(a) => {
            *state.inner.kernel.get_mut(&OUTPUT_MODE_REQUEST_MUT) = Some(OutputModeRequest::Confirm);
            // Persist to the SELECTED monitor's profile (multi-output).
            pref::upsert_output(&mut state.inner.preference.outputs, &a.edid_key, pref::ModeRequest::Advertised {
                width: a.mode.width,
                height: a.mode.height,
                refresh_mhz: a.mode.refresh_mhz,
            });
            let _ = pref::save(&state.inner.preference);
        }
        SettingsMessage::Revert => {
            *state.inner.kernel.get_mut(&OUTPUT_MODE_REQUEST_MUT) = Some(OutputModeRequest::Revert);
        }
        SettingsMessage::Rebind(id, combo) => {
            state.inner.keybinding.set(&id, combo);
            let _ = compositor_developer_environment_keybinding_base::base::save(&state.inner.keybinding);
        }
        SettingsMessage::ResetBind(id) => {
            state.inner.keybinding.clear(&id);
            let _ = compositor_developer_environment_keybinding_base::base::save(&state.inner.keybinding);
        }
        SettingsMessage::SetDefaultSink(name) => {
            if let Some(a) = state.inner.kernel.get(&AUDIO) {
                let _ = a.set_default_sink(&name);
            }
        }
        SettingsMessage::SetSinkVolume(name, v) => {
            if let Some(a) = state.inner.kernel.get(&AUDIO) {
                let _ = a.set_sink_volume(&name, v as f64);
            }
        }
        SettingsMessage::WifiEnable(b) => wifi::command(WifiCmd::SetEnabled(b)),
        SettingsMessage::WifiScan => wifi::command(WifiCmd::Scan),
        SettingsMessage::WifiConnect(ssid, pw) => wifi::command(WifiCmd::Connect(ssid, pw)),
        SettingsMessage::BtPower(b) => bt::command(BtCmd::SetPowered(b)),
        SettingsMessage::BtScan(b) => bt::command(BtCmd::Scan(b)),
        SettingsMessage::BtPair(p) => bt::command(BtCmd::Pair(p)),
        SettingsMessage::BtConnect(p) => bt::command(BtCmd::Connect(p)),
        SettingsMessage::Close => {
            state.inner.kernel.get_mut(&SETTINGS_MUT).open = false;
        }
        // Inbound / UI-local — never forwarded to the handler.
        // Tab IS forwarded (so the compositor knows the visible module): gate the
        // live-FPS push on the Performance tab being open.
        SettingsMessage::Tab(t) => {
            let st = state.inner.kernel.get_mut(&SETTINGS_MUT);
            st.fps_wanted = matches!(t, Tab::Performance);
            st.tab = t.to_index(); // remember the module for the session (restored on reopen)
            // Leaving Display abandons any provisional mode change → revert it
            // (a no-op in the kernel if nothing is pending).
            if !matches!(t, Tab::Display) {
                *state.inner.kernel.get_mut(&OUTPUT_MODE_REQUEST_MUT) = Some(OutputModeRequest::Revert);
            }
        }
        // Cursor-teleport layout committed on drag-end: persist the whole
        // arrangement and rebuild the live teleport layout so crossings take effect
        // immediately (no reboot). Layout is teleport-only — no modeset.
        SettingsMessage::LayoutCommit(placements) => {
            pref::set_layout(&mut state.inner.preference, placements);
            let _ = pref::save(&state.inner.preference);
            let keys = connected_keys(state);
            state.inner.teleport =
                compositor_orchestration_core_state_base::state::build_teleport(&state.inner.preference, &keys);
            // The tracked placement may have been removed/renumbered; re-resolve lazily.
            state.inner.cursor_placement = None;
        }
        // Activate / deactivate a monitor. `None` = deactivate ("Inactive"); refused
        // if it's the last active+connected one. `Some(mode)` = (re)activate at `mode`.
        // Persist, then ask the kernel to reconcile (bring the pipe up / tear it down);
        // reconcile rebuilds the teleport map from the new active set.
        SettingsMessage::SetActive(edid, mode_opt) => {
            match mode_opt {
                None => {
                    let active_connected = state
                        .inner
                        .kernel
                        .get(&OUTPUTS_SNAPSHOT)
                        .displays
                        .iter()
                        .filter(|d| d.connected && d.enabled)
                        .count();
                    if active_connected <= 1 {
                        return; // keep at least one active monitor
                    }
                    pref::set_active(&mut state.inner.preference.outputs, &edid, false);
                }
                Some(mode) => {
                    pref::set_active(&mut state.inner.preference.outputs, &edid, true);
                    pref::upsert_output(&mut state.inner.preference.outputs, &edid, pref::ModeRequest::Advertised {
                        width: mode.width,
                        height: mode.height,
                        refresh_mhz: mode.refresh_mhz,
                    });
                }
            }
            let _ = pref::save(&state.inner.preference);
            *state.inner.kernel.get_mut(&OUTPUT_RECONCILE_REQUEST_MUT) = true;
        }
        SettingsMessage::SetCyclic(b) => {
            state.inner.preference.teleport_cyclic = b;
            let _ = pref::save(&state.inner.preference);
            let keys = connected_keys(state);
            state.inner.teleport =
                compositor_orchestration_core_state_base::state::build_teleport(&state.inner.preference, &keys);
        }
        SettingsMessage::Fps(_)
        | SettingsMessage::ModeResult(_)
        | SettingsMessage::SyncSystem(..)
        | SettingsMessage::SyncDisplays(_)
        | SettingsMessage::SelectDisplay(_)
        | SettingsMessage::SelectMode(_)
        | SettingsMessage::SelectInactive
        // Staging an activate/deactivate is UI-local (arms the confirm bar); APPLY then
        // forwards `SetActive`.
        | SettingsMessage::StageActive(..)
        // Layout edits are applied UI-locally in the view; only LayoutCommit forwards.
        | SettingsMessage::LayoutPlace(..)
        | SettingsMessage::LayoutMove(..)
        | SettingsMessage::LayoutResize(..)
        | SettingsMessage::LayoutSelect(_)
        | SettingsMessage::LayoutRemove(_)
        | SettingsMessage::WifiSelect(_)
        | SettingsMessage::WifiPassword(_) => {}
    }
}
