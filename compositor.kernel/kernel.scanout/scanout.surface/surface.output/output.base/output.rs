//! The delegation host (Law 6): owns smithay's DrmOutputManager / DrmOutput
//! behind our typing. Sibling crates define their interfaces as if the pipe
//! were ours; the hosted objects carry the mechanism, and the (deferred)
//! de-delegation replaces them crate-by-crate.
//!
//! Failure policy: pipe bring-up must succeed (after the mode fallback chain
//! has had its say) — panic. Activate/reset keep local Results because their
//! one caller is the session-resume protocol, the designated self-recovering
//! path.

use smithay::backend::allocator::format::FormatSet;
use smithay::backend::allocator::gbm::{GbmAllocator, GbmDevice};
use smithay::backend::allocator::Fourcc;
use smithay::backend::drm::exporter::gbm::GbmFramebufferExporter;
use smithay::backend::drm::output::{DrmOutput, DrmOutputManager, DrmOutputRenderElements};
use smithay::backend::drm::{DrmDevice, DrmDeviceFd};
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::renderer::element::RenderElement;
use smithay::backend::renderer::{Bind, Renderer, Texture};
use smithay::desktop::utils::OutputPresentationFeedback;
use smithay::output::Output;
use smithay::reexports::drm::control::{connector, crtc, Mode as DrmMode};

/// The per-frame user data carried through queue_frame -> frame_submitted.
pub type FrameUserData = Option<OutputPresentationFeedback>;

/// Our names for the smithay pipe objects (the only place the full generic
/// signature is spelled out).
pub type NativeDrmOutput = DrmOutput<
    GbmAllocator<DrmDeviceFd>,
    GbmFramebufferExporter<DrmDeviceFd>,
    FrameUserData,
    DrmDeviceFd,
>;
pub type NativeDrmOutputManager = DrmOutputManager<
    GbmAllocator<DrmDeviceFd>,
    GbmFramebufferExporter<DrmDeviceFd>,
    FrameUserData,
    DrmDeviceFd,
>;

/// The color formats offered to the pipe. With `ten_bit`, 10-bit formats are
/// listed first and the 8-bit formats kept as a fallback — smithay negotiates
/// the first the plane supports, so a panel that can't scan out 10-bit falls
/// back to 8-bit rather than failing. `ten_bit` is requested both by HDR (PQ
/// needs the extra precision) and by plain deep-color SDR (COMPOSITOR_DEPTH=10):
/// the format choice is independent of the transfer function — 10-bit SDR scans
/// out the same sRGB values at finer quantization (less banding), no PQ.
pub fn color_formats(ten_bit: bool) -> Vec<Fourcc> {
    if ten_bit {
        vec![
            Fourcc::Xrgb2101010,
            Fourcc::Argb2101010,
            Fourcc::Argb8888,
            Fourcc::Abgr8888,
        ]
    } else {
        vec![Fourcc::Argb8888, Fourcc::Abgr8888]
    }
}

pub fn manager(
    drm: DrmDevice,
    allocator: GbmAllocator<DrmDeviceFd>,
    exporter: GbmFramebufferExporter<DrmDeviceFd>,
    gbm: Option<GbmDevice<DrmDeviceFd>>,
    render_formats: FormatSet,
    ten_bit: bool,
) -> NativeDrmOutputManager {
    DrmOutputManager::new(
        drm,
        allocator,
        exporter,
        gbm,
        color_formats(ten_bit).into_iter(),
        render_formats,
    )
}

/// Bring the pipe online (ex wire.rs `new()` step 8). Returns Err so the mode
/// fallback chain (`commit.test`) can try the next candidate; the CHAIN
/// exhausting is the panic, at the assembly site.
#[allow(clippy::too_many_arguments)]
pub fn initialize<R, E>(
    manager: &mut NativeDrmOutputManager,
    pipe: crtc::Handle,
    mode: DrmMode,
    connectors: &[connector::Handle],
    output: &Output,
    renderer: &mut R,
) -> Result<NativeDrmOutput, String>
where
    // The vendored smithay's `initialize_output` requires these bounds on the
    // renderer; propagate them onto our delegation wrapper's `R`.
    R: Renderer + Bind<Dmabuf>,
    R::TextureId: Texture + 'static,
    R::Error: Send + Sync + 'static,
    E: RenderElement<R>,
{
    manager
        .lock()
        .initialize_output::<_, E>(
            pipe,
            mode,
            connectors,
            output,
            None,
            renderer,
            &DrmOutputRenderElements::default(),
        )
        .map_err(|e| format!("initialize_output failed: {e:?}"))
}

/// Session-pause the whole device's pipes.
pub fn pause(manager: &mut NativeDrmOutputManager) {
    manager.pause();
}

/// Session-activate; `force = true` performs the reclaiming modeset.
/// Result is for the resume protocol (self-recovering class).
pub fn activate(manager: &mut NativeDrmOutputManager, force: bool) -> Result<(), String> {
    manager
        .lock()
        .activate(force)
        .map_err(|e| format!("DRM activate failed: {e:?}"))
}

/// DPMS power the surface's connectors on/off without tearing down the pipe
/// (lid-close blank, idle). Recover an off display with `activate(force=true)`
/// + `reset`, exactly like the resume path. NOTE: any page-flip while off
/// re-powers the connector (legacy DPMS auto-on on commit), so the render loop
/// must be gated off in tandem.
pub fn set_dpms(output: &NativeDrmOutput, on: bool) -> Result<(), String> {
    let mut result = Ok(());
    output.with_compositor(|compositor| {
        if let Err(err) = compositor.surface().set_dpms(on) {
            result = Err(format!("surface set_dpms({on}) failed: {err:?}"));
        }
    });
    result
}

/// Reset the running surface state + buffers (resume steps 3-4).
/// Result is for the resume protocol (self-recovering class).
pub fn reset(output: &mut NativeDrmOutput) -> Result<(), String> {
    let mut result = Ok(());
    output.with_compositor(|compositor| {
        if let Err(err) = compositor.surface().reset_state() {
            result = Err(format!("surface reset_state failed: {err:?}"));
        }
    });
    output.reset_buffers();
    result
}
