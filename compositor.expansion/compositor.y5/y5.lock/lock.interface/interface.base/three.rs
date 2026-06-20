use smithay::{
    backend::renderer::gles::GlesRenderer,
    utils::{Physical, Point, Size},
};
use compositor_support_bevy_core_compositor_base::BevyHandle;
use compositor_background_three_lock_scene::MorphScene;
use compositor_orchestration_core_state_base::Loop;
use compositor_y5_graphic_capture_registry::SnapshotHandle;
use compositor_y5_lock_state_base::state::LockActiveCapture;

pub(crate) fn create(
    state: &mut Loop,
    renderer: &mut GlesRenderer,
    size: Size<i32, Physical>,
) -> Option<BevyHandle<MorphScene>> {
    let snap: Option<compositor_y5_graphic_capture_registry::SnapshotHandle> =
        get_snapshot(state, renderer, size);

    let mut active = state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().get_mut(&compositor_y5_lock_system_base::base::LOCK_MUT).active.clone().unwrap_or_else(|| abort!("locked"));
    let Some(snap) = snap else {
        active.capture = LockActiveCapture::None;
        return None;
    };

    // The LOCK world OWNS its bevy registry, pre-created at startup by the loader
    // prewarm pass — asserted present here rather than built mid-render.
    let Some(registry) = state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().try_get_mut(&compositor_background_three_system_base::base::BG_THREE_MUT).and_then(|b| b.registry.as_mut()) else {
        abort!("lock: bevy registry missing — startup prewarm failed");
    };

    let wgpu = snap.wgpu_texture();
    active.capture = compositor_y5_lock_state_base::state::LockActiveCapture::Snapshot(snap);

    // Create one lock-screen morph instance at full output size, in screen
    // space so it stays fixed regardless of camera transform.
    let Ok(handle) = registry.create_screen(
        &state.inner.environment.GPU.as_str(),
        MorphScene::new((size.w as u32, size.h as u32), wgpu),
        renderer,
        Point::from((0, 0)),
        size,
        compositor_orchestration_draw_layer_base::base::Layer::LOCK_SCENE.bits(),
    ) else {
        active.capture = compositor_y5_lock_state_base::state::LockActiveCapture::None;
        return None;
    };

    match registry.dispatch_command(
        handle,
        compositor_background_three_lock_scene::MorphCommand::Lock,
    ) {
        Ok(_) => {}
        Err(e) => {
            error!("dispatch lock command failed: {}", e);
            active.capture = compositor_y5_lock_state_base::state::LockActiveCapture::None;
            return None;
        }
    }

    state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().get_mut(&compositor_y5_lock_system_base::base::LOCK_MUT).active = Some(active);
    return Some(handle);
}

fn get_snapshot(
    state: &mut Loop,
    renderer: &mut GlesRenderer,
    size: Size<i32, Physical>,
) -> Option<SnapshotHandle> {
    let Some(active) = &state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().get_mut(&compositor_y5_lock_system_base::base::LOCK_MUT).active else {
        abort!()
    };

    let LockActiveCapture::Capture(Handle) = &active.capture else {
        return None;
    };

    Handle
        .snapshot(&state.inner.environment.GPU.as_str(), renderer)
        .ok()
}
