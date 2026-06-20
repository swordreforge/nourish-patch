use std::os::unix::raw::dev_t;

use compositor_y5_graphic_display_backend::backend::Backend;
use smithay::{output::Output, wayland::dmabuf::DmabufFeedbackBuilder};
use compositor_orchestration_core_state_base::Loop;

pub fn register(_loop: &mut Loop, output: &Output) {
    let _global = output.create_global::<compositor_support_smithay_dispatch_state_base::state::Dispatch>(&_loop.state.output.display_handle);

    _loop.inner.space_state_mut().state.map_output(output, (0, 0));
}

pub fn register_dmabuf(_loop: &mut Loop, backend_loader: &mut dyn Backend) {
    let dma_formats = backend_loader.bind_display(&_loop.state.output.display_handle);

    if _loop.inner.kernel.get(&compositor_orchestration_core_state_base::state::GPU_BINDING).is_some() {
        info!("Creating DMABuf global v5");
        // Get the primary GPU's device node.
        // backend_loader needs to expose this — typically the primary_gpu DrmNode or its dev_id.
        let main_device: dev_t = _loop.inner.kernel.get(&compositor_orchestration_core_state_base::state::GPU_BINDING)
            .as_ref()
            .unwrap()
            .borrow()
            .primary
            .dev_id();
        // let main_device: dev_t = _loop.inner.kernel.get(&compositor_orchestration_core_state_base::state::GPU_BINDING).unwrap().borrow_mut().primary.dev_id();

        // Build the default feedback.
        let default_feedback = DmabufFeedbackBuilder::new(main_device, dma_formats)
            .build()
            .unwrap_or_else(|e| abort!("Failed to build dmabuf feedback: {e:?}"));

        _loop.state.dmabuf.global = Some(
            _loop
                .state
                .dmabuf
                .state
                .create_global_with_default_feedback::<compositor_support_smithay_dispatch_state_base::state::Dispatch>(
                    &_loop.state.output.display_handle,
                    &default_feedback,
                ),
        );
    } else {
        warn!("Creating DMABuf global V3 instead of V5 because primary gpu is not set");
        _loop.state.dmabuf.global = Some(
            _loop
                .state
                .dmabuf
                .state
                .create_global::<compositor_support_smithay_dispatch_state_base::state::Dispatch>(&_loop.state.output.display_handle, dma_formats),
        );
    }
}
