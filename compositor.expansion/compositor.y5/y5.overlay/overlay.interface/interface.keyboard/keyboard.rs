use smithay::backend::input::{InputBackend, KeyState};
use smithay::backend::session::Session;
use smithay::input::keyboard::{Keysym, ModifiersState};
use compositor_support_library_input_keyboard_base::keyboard::combo::KeyCombo;
use compositor_support_library_input_keyboard_base::keyboard::handler::ShortcutHandler;
use compositor_support_library_input_keyboard_base::keyboard::key::Key;
use compositor_support_library_input_keyboard_base::shortcut;

use compositor_orchestration_core_state_base::Loop;

pub fn input_received<I: InputBackend>(
    state: &mut Loop,
    keysym: Keysym,
    key_state: KeyState,
    modifiers: &ModifiersState,
) -> bool {
    let key = Key::from_keysym(keysym);
    if key.is_none() {
        return false;
    }

    let is_press = key_state == KeyState::Pressed;
    // Only handle presses.
    if !is_press {
        return false;
    }

    // Build the vector manually using standard Rust struct initialization.
    // We only use the single `shortcut!` macro to generate the KeyCombo.
    let handlers: Vec<ShortcutHandler<Loop>> = inline_shortcut_handlers(state.inner.storage.nested);

    for handler in handlers {
        if handler.combo.matches(modifiers, key) {
            if (handler.action)(state) {
                return true;
            }
        }
    }

    // Add Playback device control, TTY switches.
    return false;
}

// CHECK:
// THe compositor process sets active kernal mode: K_OFF or similar, which causes it to not handle TTY switches.
// It meants tty switches must be handled here if needed,
// but kernel sends the TTY shortcuts under different keysyms.
// Reference: ""One thing to verify in your handler — VT keysyms come through xkb as XKB_KEY_XF86Switch_VT_1 … XF86Switch_VT_12 when the Ctrl+Alt level is active, so it's often cleaner to match on those keysyms than to reconstruct "Ctrl+Alt+F1" from modifiers + F-key yourself. Match the XF86Switch_VT_N syms, extract N, call change_vt(N).
const VOLUME_STEP: f64 = 0.05;
pub fn inline_shortcut_handlers(nosuper: bool) -> Vec<ShortcutHandler<Loop>> {
    let mut handlers: Vec<ShortcutHandler<Loop>> = vec![
        // TEMPORARY (sanity test): switch the active/spawn-target world. Behaves as
        // "upsert + change" against pre-created spatial test worlds, to exercise
        // world delegation until real world selection lands.
        ShortcutHandler {
            combo: shortcut!(Super + Alt + Num1),
            action: Box::new(move |s| switch_world(s, 0)),
        },
        ShortcutHandler {
            combo: shortcut!(Super + Alt + Num2),
            action: Box::new(move |s| switch_world(s, 1)),
        },
        ShortcutHandler {
            combo: shortcut!(Super + Alt + Num3),
            action: Box::new(move |s| switch_world(s, 2)),
        },
        ShortcutHandler {
            combo: shortcut!(Super + Alt + L),
            action: Box::new(move |s| {
                sleep(s);
                true
            }),
        },
        // World-selection screen: SUPER+K opens it (or cancels it if already up).
        ShortcutHandler {
            combo: shortcut!(Super + K),
            action: Box::new(move |s| {
                compositor_y5_picker_interface_entry::entry::toggle(s);
                true
            }),
        },
        // Escape cancels the picker — but only while it's showing, so the
        // binding falls through (returns false) for every other world.
        ShortcutHandler {
            combo: shortcut!(Escape),
            action: Box::new(move |s| {
                if s.inner.worlds.active_id()
                    == compositor_y5_picker_system_base::base::PICKER_WORLD
                {
                    compositor_y5_picker_interface_base::base::cancel(s);
                    true // (cancel lives in interface.base)
                } else {
                    false
                }
            }),
        },
        ShortcutHandler {
            combo: shortcut!(Ctrl + Alt + SwitchVt1),
            action: Box::new(move |s| {
                tty(s, 1);
                true
            }),
        },
        ShortcutHandler {
            combo: shortcut!(Ctrl + Alt + SwitchVt2),
            action: Box::new(move |s| {
                tty(s, 2);
                true
            }),
        },
        ShortcutHandler {
            combo: shortcut!(Ctrl + Alt + SwitchVt3),
            action: Box::new(move |s| {
                tty(s, 3);
                true
            }),
        },
        ShortcutHandler {
            combo: shortcut!(Ctrl + Alt + SwitchVt4),
            action: Box::new(move |s| {
                tty(s, 4);
                true
            }),
        },
        ShortcutHandler {
            combo: shortcut!(Ctrl + Alt + SwitchVt5),
            action: Box::new(move |s| {
                tty(s, 5);
                true
            }),
        },
        ShortcutHandler {
            combo: shortcut!(Ctrl + Alt + SwitchVt6),
            action: Box::new(move |s| {
                tty(s, 6);
                true
            }),
        },
        ShortcutHandler {
            combo: shortcut!(AudioRaiseVolume),
            action: Box::new(|s| {
                if let Some(audio) = s.inner.kernel.get_mut(&compositor_orchestration_driver_audio_base::base::AUDIO_MUT) {
                    let _ = audio.adjust_volume(VOLUME_STEP);
                }
                true // consumed — do not forward to clients
            }),
        },
        ShortcutHandler {
            combo: shortcut!(AudioLowerVolume),
            action: Box::new(|s: &mut Loop| {
                if let Some(audio) = s.inner.kernel.get_mut(&compositor_orchestration_driver_audio_base::base::AUDIO_MUT) {
                    let _ = audio.adjust_volume(-VOLUME_STEP);
                }

                true
            }),
        },
        ShortcutHandler {
            combo: shortcut!(AudioMute),
            action: Box::new(|s: &mut Loop| {
                if let Some(audio) = s.inner.kernel.get_mut(&compositor_orchestration_driver_audio_base::base::AUDIO_MUT) {
                    let _ = audio.toggle_mute();
                }

                true
            }),
        },
        // Transport keys. These belong to your MPRIS layer (the deferred notch work).
        // Until that exists, register them so they're consumed rather than leaking to
        // the focused client; swap the bodies for `s.media.play_pause()` etc. later.
        ShortcutHandler {
            combo: shortcut!(AudioPlay), // single play key = play/pause toggle
            action: Box::new(|s| {
                if let Some(media) = s.inner.kernel.get_mut(&compositor_orchestration_driver_audio_base::base::MEDIA_MUT) {
                    let _ = media.play_pause();
                }
                /* TODO: active_player.play_pause() */
                true
            }),
        },
        ShortcutHandler {
            combo: shortcut!(AudioPause),
            action: Box::new(|s| {
                if let Some(media) = s.inner.kernel.get_mut(&compositor_orchestration_driver_audio_base::base::MEDIA_MUT) {
                    let _ = media.pause();
                }

                true
            }),
        },
        ShortcutHandler {
            combo: shortcut!(AudioStop),
            action: Box::new(|s| {
                if let Some(media) = s.inner.kernel.get_mut(&compositor_orchestration_driver_audio_base::base::MEDIA_MUT) {
                    let _ = media.stop();
                }
                true
            }),
        },
        ShortcutHandler {
            combo: shortcut!(AudioNext),
            action: Box::new(|s| {
                if let Some(media) = s.inner.kernel.get_mut(&compositor_orchestration_driver_audio_base::base::MEDIA_MUT) {
                    let _ = media.next();
                }

                true
            }),
        },
        ShortcutHandler {
            combo: shortcut!(AudioPrev),
            action: Box::new(|s| {
                if let Some(media) = s.inner.kernel.get_mut(&compositor_orchestration_driver_audio_base::base::MEDIA_MUT) {
                    let _ = media.previous();
                }

                true
            }),
        },
    ];

    if nosuper {
        for w in &mut handlers {
            if w.combo.modifiers.logo {
                w.combo.modifiers.logo = false;
                w.combo.modifiers.ctrl = true;
            }
        }
    }

    handlers
}

fn tty(state: &mut Loop, num: u32) {
    error!("VT switch to {:?}", num);
    state.loop_handle.insert_idle(move |state| {
        // Do not perform the action again when paused.
        if let compositor_orchestration_core_state_base::state::StatusSession::Paused =
            state.inner.status_session
        {
            error!("VT already paused");
            return;
        }

        let Some(tty) = &mut state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().get_mut(&compositor_y5_lock_system_base::base::LOCK_MUT).tty else {
            error!("VT unavailable");
            return;
        };

        if let Some(ses) = &mut state.state.seat.libseat {
            ses.change_vt(num as i32);
        }
        // match tty.switch_to(num) {
        //     Ok(()) => {
        //         tracing::error!("VT switch request success");
        //         state.inner.status_session =
        //             compositor_orchestration_core_state_base::state::StatusSession::Paused;
        //     }
        //     Err(e) => {
        //         tracing::error!("VT switch to {num} failed: {e:?}");
        //         // Switch didn't happen, undo the pause flag.
        //     }
        // }
    });
}

fn sleep(state: &mut Loop) {
    error!("Sleep");
    state.loop_handle.insert_idle(move |state| {
        state.inner.__set_lock =
            Some(compositor_orchestration_core_state_base::state::SetLockRequest { sleep: true });
    });
}

/// TEMPORARY (sanity test): make test-world `slot` (0=main, 1/2=pre-created
/// spatial worlds) the active + spawn-target world, exercising world delegation.
/// Maps the output into the target world's space on first entry so it renders.
fn switch_world(state: &mut Loop, slot: usize) -> bool {
    let target = state.inner.kernel.get(&compositor_orchestration_core_state_base::state::TEST_WORLDS)[slot];
    // The output handle lives on the current spawn-target space; grab it before switching.
    let output = state.inner.space_state().state.outputs().next().cloned();

    state.inner.worlds.switch(target, &state.inner.kernel);
    state.inner.worlds.set_spawn_target(target);
    info!("world switch -> slot {slot} (world {target})");

    // First time we enter a test world its space has no output mapped; map it so
    // the backend renders this world's windows.
    if let Some(output) = output {
        if state.inner.space_state().state.outputs().next().is_none() {
            state
                .inner
                .space_state_mut()
                .state
                .map_output(&output, smithay::utils::Point::from((0, 0)));
        }
    }
    true
}

// // // Executes both lock and sleep.
// fn sleep(s: &mut Loop) {
//     // There is an "inhibitor" way to way for lock which is better.
//     lock(s);
//     // s.wait_until_locked(Duration::from_millis(500));
//
//     let _ = Command::new("systemctl")
//         .arg("suspend")
//         .spawn(); // fire and forget; suspend doesn't return until resume
// }
//
// // Completely terminate the session and all process within it.
// // It needs to be like regular gnome logout.
