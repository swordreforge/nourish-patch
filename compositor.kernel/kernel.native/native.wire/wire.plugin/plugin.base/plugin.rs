//! Device plug/unplug wiring: registers the retained udev watcher and routes
//! typed events to the host's reaction (`native.plugin/plugin.route`).
//! NEW capability of the restructure: previously the UdevBackend was dropped
//! after the initial lookup and hotplug events never flowed.

use compositor_kernel_gpu_registry_node_base::node::NodeRegistry;
use compositor_kernel_native_context_render_base::render::NativeRenderContext;
use compositor_kernel_native_context_topology_base::topology::Topology;
use compositor_kernel_udev_loop_watch_base::watch::UdevWatch;
use smithay::reexports::calloop::EventLoop;
use std::cell::RefCell;
use std::rc::Rc;
use compositor_orchestration_core_state_base::Loop;

pub fn register(
    event_loop: &mut EventLoop<'static, Loop>,
    watch: UdevWatch,
    registry: Rc<RefCell<NodeRegistry>>,
    topology: Rc<RefCell<Topology>>,
    ctx_rc: Rc<RefCell<NativeRenderContext>>,
) {
    event_loop
        .handle()
        .insert_source(watch, move |event, _, state| {
            let decoded = compositor_kernel_udev_loop_event_base::event::decode(event);
            let rank = compositor_kernel_graphic_preference_gpu_rank::rank::get();
            compositor_kernel_native_plugin_route_base::route::route(
                decoded,
                &mut registry.borrow_mut(),
                &mut topology.borrow_mut(),
                &rank,
                &ctx_rc,
            );
            // Refresh the rim's display snapshot after any topology change so the
            // lid policy sees current external/internal presence.
            let active = ctx_rc.borrow().connector;
            let ctx = ctx_rc.borrow();
            let manager = ctx.drm_output_manager.borrow();
            let snap = compositor_kernel_native_context_display_base::base::compute(
                manager.device(),
                active,
            );
            drop(manager);
            drop(ctx);
            *state
                .inner
                .kernel
                .get_mut(&compositor_orchestration_driver_lid_base::base::DISPLAY_SNAPSHOT_MUT) =
                snap;
        })
        .unwrap();
}
