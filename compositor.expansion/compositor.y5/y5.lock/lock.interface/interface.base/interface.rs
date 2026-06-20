use std::time::Duration;
use std::time::Instant;

use crate::surface;
use crate::three;
use smithay::backend::input::KeyState;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::input::keyboard::FilterResult;
use smithay::input::keyboard::ModifiersState;
use smithay::reexports::calloop::timer::TimeoutAction;
use smithay::reexports::calloop::timer::Timer;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::SERIAL_COUNTER;
use smithay::utils::{Physical, Point, Scale, Size};
use compositor_orchestration_core_state_base::state::Status;
use compositor_orchestration_core_state_base::Loop;
use compositor_y5_graphic_capture_registry::{CaptureSource, OutputId};
use compositor_y5_lock_state_base::state::{LockActiveCapture, LockActiveState};
use compositor_monitor_compositor_iced_base::IcedHandle;

pub fn lock(state: &mut Loop, renderer: &mut GlesRenderer, size: Size<i32, Physical>, sleep: bool) {
    if let compositor_orchestration_core_state_base::state::Status::Locked { .. } = state.inner.status {
        error!("lock when already locked");
        return;
    }

    // Stop and discard any in-progress capture before the lock takes its own
    // snapshot, so the capture overlays don't bleed into the lock background.
    compositor_y5_graphic_capture_interface::interface::stop_and_discard(state);

    compositor_y5_navigator_interface_base::interface::lock(state);

    let mut surface_element = vec![];

    let mut surface_input: Option<
        IcedHandle<compositor_y5_lock_interface_surface::view::LockSurface>,
    > = None;

    if let Some(surface) = surface::create(state, renderer, size) {
        surface_element.push(surface.id);
        surface_input = Some(surface);
    }

    let capture = if let Some(registry_capture) = state.inner.kernel.get_mut(&compositor_orchestration_driver_capture_base::base::CAPTURE_REGISTRY_MUT).as_mut() {
        if let Some(cap) = registry_capture
            .request(&state.inner.environment.GPU.as_str(), renderer, CaptureSource::OutputFramebuffer(OutputId(0)))
            .ok()
        {
            LockActiveCapture::Capture(cap)
        } else {
            LockActiveCapture::None
        }
    } else {
        LockActiveCapture::None
    };

    let active = LockActiveState {
        bevy: None,
        capture,
        surface: surface_element,
        surface_input,
    };

    state.inner.status = compositor_orchestration_core_state_base::state::Status::Locked {
        pending: true,
        sleep,
        time: Instant::now(),
    };
    // Locking IS a world switch: session systems get on_disable, lock systems
    // on_enable. The Status enum stays alongside until the legacy scene/input
    // selection migrates onto the active world (then it dissolves).
    {
        let (worlds, kernel) = (&mut state.inner.worlds, &state.inner.kernel);
        worlds.switch(compositor_y5_lock_system_base::base::LOCK_WORLD, kernel);
    }

    state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().get_mut(&compositor_y5_lock_system_base::base::LOCK_MUT).active = Some(active);

    deactivate_scene(state);
}

fn deactivate_scene(_loop: &mut Loop) {
    // Before dropping focus, release held (non-modifier) keys to the focused client so a client
    // that tracks its own keyboard state doesn't get stuck key-down on re-focus after the lock.
    compositor_orchestration_seat_keyboard_input::keyboard::release_held_keys(_loop);

    let keyboard = _loop.state.seat.seat.get_keyboard().unwrap();
    let serial = SERIAL_COUNTER.next_serial();

    // Deactivate keyboard focus
    keyboard.set_focus(&mut _loop.state, Option::<WlSurface>::None, serial);

    _loop.inner.space_state().state.elements().for_each(|window| {
        window.set_activated(false);
        window.toplevel().unwrap().send_pending_configure();
    });

    // CHECK: Ice Registry should be per scene, as well as space, etc.
    //        this is due to occur when space is encapsulated ( layers feature ).
    // CHECK: IcedRegistry has pointer_grab. it should probably be released as well.
    if let Some(registry) = _loop.inner.surface_mut().registry.as_mut() {
        // Iced deactivation:
        registry.set_keyboard_focus(None);
        // registry.dispatch_button(None, button, true);
    }

    clear_keyboard(_loop);
}
fn clear_keyboard(_loop: &mut Loop) {
    let keyboard = &_loop.state.seat.seat.get_keyboard().unwrap();
    let serial = SERIAL_COUNTER.next_serial();

    let now = _loop.inner.start_time.elapsed().as_millis() as u32;
    for key in keyboard.pressed_keys() {
        // confirm accessor name in your tree
        keyboard.input::<(), _>(
            &mut _loop.state,
            key, // Keycode
            KeyState::Released,
            SERIAL_COUNTER.next_serial(),
            now,
            |_, _, _| FilterResult::Intercept(()), // update internal state, don't forward
        );
    }

    // 3. Clear modifier state directly (belt-and-suspenders).
    keyboard.set_modifier_state(ModifiersState::default());
}

pub fn lock_done(state: &mut Loop, renderer: &mut GlesRenderer, size: Size<i32, Physical>) {
    let compositor_orchestration_core_state_base::state::Status::Locked { sleep, time, .. } =
        state.inner.status
    else {
        abort!("Lock done while not locked")
    };

    if state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().get_mut(&compositor_y5_lock_system_base::base::LOCK_MUT).active.is_none() {
        abort!("Lock done while not locked ( no active set )");
    }

    // Progress to locked complete
    state.inner.status = compositor_orchestration_core_state_base::state::Status::Locked {
        pending: false,
        sleep: false,
        time,
    };

    // Create the bevy handle
    let bevy = three::create(state, renderer, size);
    let Some(ref mut active) = &mut state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().get_mut(&compositor_y5_lock_system_base::base::LOCK_MUT).active else {
        abort!();
    };
    active.bevy = bevy;

    // set lock time on ParallaxBackground (the session world being locked ==
    // spawn_target; locking only moved `active` to LOCK_WORLD).
    let session = state.inner.worlds.spawn_target();
    if let Some(ref mut instance) = state.inner.worlds.get_mut(session).storage_mut().get_mut(&compositor_background_two_system_base::base::BG_TWO_MUT).instance {
        instance.lock_time = Some(Instant::now());
        instance.pan = (0.0, 0.0);
        instance.zoom = 1.0;
    }

    state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().get_mut(&compositor_y5_lock_system_base::base::LOCK_MUT).pam = crate::pam::make_pam(&state.loop_handle);

    // It was set as sleep, so insert a timer to callback after period to lock
    if sleep {
        // CHECK: THis isn't the actual period for the animation of morph
        let period = compositor_y5_lock_state_transition::transition::PERIOD * 2.0;
        let delay = Duration::from_secs_f64(period);
        let timer_source = Timer::from_duration(delay);

        // 2. Insert it into the loop handle
        let _ = state
            .loop_handle
            .insert_source(timer_source, |deadline, _metadata, state| {
                // IT should block shortcuts already.
                let Some(tty) = &mut state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().get_mut(&compositor_y5_lock_system_base::base::LOCK_MUT).tty else {
                    return TimeoutAction::Drop;
                };
                let _ = tty.suspend();

                TimeoutAction::Drop
            });
    }
}

pub fn unlock_fail(state: &mut Loop) {
    info!("Unlock");
    match state.inner.status {
        compositor_orchestration_core_state_base::state::Status::Locked { .. } => {}
        _ => return,
    }

    let handle = state
        .inner
        .worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().get_mut(&compositor_y5_lock_system_base::base::LOCK_MUT)
        .active
        .as_ref()
        .unwrap()
        .surface_input
        .unwrap();
    let registry = state.inner.surface_mut().registry.as_mut().unwrap();
    registry.dispatch_message(
        handle,
        compositor_y5_lock_interface_surface::message::LockMessage::AuthFailed(String::from(
            "Invalid",
        )),
    );
}
pub fn unlock(state: &mut Loop) {
    info!("Unlock");
    match state.inner.status {
        compositor_orchestration_core_state_base::state::Status::Locked { .. } => {}
        _ => return,
    }

    let reg = state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().get_mut(&compositor_y5_lock_system_base::base::LOCK_MUT).pam.as_ref().and_then(|w| Some(w.1));
    if let Some(reg) = reg {
        state.loop_handle.remove(reg);
    }

    state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().get_mut(&compositor_y5_lock_system_base::base::LOCK_MUT).pam = None;

    let destroy_ids = {
        let active = &state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().get_mut(&compositor_y5_lock_system_base::base::LOCK_MUT).active.as_ref().unwrap();
        active.surface.clone()
    };

    if let Some(registry) = state.inner.surface_mut().registry.as_mut() {
        for item in destroy_ids {
            registry.destroy_by_id(item);
        }
    }

    let destroy_ids = {
        let active = &state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().get_mut(&compositor_y5_lock_system_base::base::LOCK_MUT).active.as_ref().unwrap();
        active.bevy.and_then(|w| Some(w.id)).clone()
    };

    // The morph instance lives in the LOCK world's OWN registry now.
    if let Some(registry) = state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().try_get_mut(&compositor_background_three_system_base::base::BG_THREE_MUT).and_then(|b| b.registry.as_mut()) {
        if let Some(bevy) = destroy_ids {
            registry.destroy_by_id(bevy);
        }
    }

    let session = state.inner.worlds.spawn_target();
    if let Some(bg) = &mut state.inner.worlds.get_mut(session).storage_mut().get_mut(&compositor_background_two_system_base::base::BG_TWO_MUT).instance {
        bg.lock_time = None;
    }

    state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().get_mut(&compositor_y5_lock_system_base::base::LOCK_MUT).active = None;
    // state.inner.status = Status::Unlock {
    //     time: Instant::now(),
    // };

    state.inner.status = Status::Running;
    // Unlocking switches back to the session world (kept intact while locked):
    // spawn_target still names it, since locking only moved `active`.
    {
        let (worlds, kernel) = (&mut state.inner.worlds, &state.inner.kernel);
        let session = worlds.spawn_target();
        worlds.switch(session, kernel);
    }

    clear_keyboard(state);
    compositor_y5_navigator_interface_base::interface::unlock(state);

    // for window in _loop.inner.space_state().state.elements() {
    //     window.set_activated(false);
    //     if let Some(toplevel) = window.toplevel() {
    //         toplevel.send_pending_configure();
    //     }
    // }
}
