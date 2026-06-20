use std::time::Instant;
use compositor_support_bevy_core_compositor_base::BevyHandle;
use compositor_background_three_lock_scene::MorphScene;

pub struct Three {
    // The wgpu context is shared driver data now (kernel `BEVY_CONTEXT`), not
    // per-world; this world owns only its registry, built lazily from it.
    pub shared: Option<compositor_support_bevy_core_compositor_base::SharedContext>,
    pub registry: Option<compositor_support_bevy_core_compositor_base::BevyRegistry>,
    pub test_example_done: bool,
    pub example: Vec<
        compositor_support_bevy_core_compositor_base::BevyHandle<
            compositor_background_three_lock_scene::MorphScene,
        >,
    >,

    pub example_lock_tick: Instant,
    pub example_lock_done: bool,
    pub example_capture: Option<compositor_y5_graphic_capture_registry::CaptureHandle>,
    pub example_capture_snapshot: Option<compositor_y5_graphic_capture_registry::SnapshotHandle>,
}

impl Three {
    pub fn new() -> Self {
        Self {
            example_lock_done: false,
            example_lock_tick: Instant::now(),
            registry: None,
            shared: None,
            test_example_done: false,
            example: Vec::new(),
            // pending_snapshot_capture: None,
            example_capture: None,
            example_capture_snapshot: None,
        }
    }
}
