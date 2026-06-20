//! DRM render node → gbm BO → Dmabuf.
//!
//! This is the bottom of the import stack. `allocate_dmabuf` returns an
//! `AllocatedDmabuf` that owns the gbm device + buffer object. Dropping it
//! releases the underlying GPU memory; keep it alive for as long as either
//! WGPU or GLES holds an imported view.
//!
//! The returned `Dmabuf` is cheap to clone (it's just plane fds + metadata),
//! and the `AllocatedDmabuf` wrapper holds the lifetime-critical pieces.

use std::os::fd::{AsRawFd, OwnedFd};
use std::path::Path;

use crate::error::AllocError;
use gbm::{BufferObjectFlags, Device as GbmDevice, Format as GbmFormat};
use smithay::backend::allocator::dmabuf::{Dmabuf, DmabufFlags};
use smithay::backend::allocator::{Buffer, Fourcc};

/// Opaque holder for an allocated buffer. Keeps gbm alive while the dmabuf
/// is in use. Drop order inside this struct: `dmabuf` first (releases fds
/// and any imports), then `_bo`, then `_gbm`. Rust drops struct fields in
/// declaration order, so the ordering below is load-bearing — don't reorder.
pub struct AllocatedDmabuf {
    pub dmabuf: Dmabuf,
    // The buffer object holds the GPU allocation; dropping it before the
    // dmabuf would close handles the dmabuf still references.
    _bo: gbm::BufferObject<()>,
    // The gbm device must outlive any BO created from it.
    _gbm: GbmDevice<OwnedFd>,
}

impl std::fmt::Debug for AllocatedDmabuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AllocatedDmabuf")
            .field("size", &self.dmabuf.size())
            .field("format", &self.dmabuf.format())
            .field("num_planes", &self.dmabuf.num_planes())
            .finish()
    }
}

/// Path to the DRM render node we use for allocations.
///
/// Exposed as a constant so callers can verify or override. `/dev/dri/renderD129`
/// is the second render node on systems with multiple GPUs (NVIDIA + iGPU);
/// adjust for single-GPU systems if you find this wrong.
// pub const DEFAULT_RENDER_NODE: &str = GPU_DEVICE;

/// Allocate a single ARGB8888 LINEAR dmabuf at the given size.
///
/// LINEAR is the safest modifier for cross-API sharing (Mesa/NVIDIA/GLES/Vulkan
/// all handle it). Returns an `AllocatedDmabuf` that owns the gbm device and
/// buffer object; the inner `Dmabuf` can be cloned cheaply and imported by
/// wgpu or GLES.
pub fn allocate_dmabuf(
    render_node: &str,
    width: u32,
    height: u32,
) -> Result<AllocatedDmabuf, AllocError> {
    allocate_dmabuf_on(Path::new(render_node), width, height)
}

/// Variant of [`allocate_dmabuf`] with an explicit render node path.
pub fn allocate_dmabuf_on(
    render_node: &Path,
    width: u32,
    height: u32,
) -> Result<AllocatedDmabuf, AllocError> {
    if width == 0 || height == 0 {
        return Err(AllocError::InvalidDimensions { width, height });
    }

    // 1. Open the render node.
    let drm_file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(render_node)
        .map_err(AllocError::OpenDrm)?;
    let drm_fd: OwnedFd = drm_file.into();

    info!(
        "Opened DRM render node {} (fd={})",
        render_node.display(),
        drm_fd.as_raw_fd()
    );

    // 2. Wrap in a gbm device.
    let gbm = GbmDevice::new(drm_fd).map_err(AllocError::GbmInit)?;

    // 3. Allocate the buffer object. RENDERING usage = usable as render target.
    let bo = gbm
        .create_buffer_object::<()>(
            width,
            height,
            GbmFormat::Argb8888,
            BufferObjectFlags::RENDERING,
        )
        .map_err(AllocError::CreateBo)?;

    info!(
        "Allocated gbm BO {}x{}, format=ARGB8888, modifier={:?}",
        width,
        height,
        bo.modifier(),
    );

    // 4. Export plane(s) as a Smithay Dmabuf.
    let plane_count = bo.plane_count();
    let modifier = bo.modifier();

    let mut builder = Dmabuf::builder(
        (width as i32, height as i32),
        Fourcc::Argb8888,
        modifier,
        DmabufFlags::empty(),
    );

    for plane in 0..plane_count {
        let fd = bo
            .fd_for_plane(plane as i32)
            .map_err(AllocError::ExportFd)?;
        let offset = bo.offset(plane as i32);
        let stride = bo.stride_for_plane(plane as i32);

        builder.add_plane(fd, plane as u32, offset, stride);
    }

    let dmabuf = builder.build().ok_or(AllocError::BuildDmabuf)?;

    info!(
        "Built Dmabuf: size={:?}, format={:?}, num_planes={}, modifier={:?}",
        dmabuf.size(),
        dmabuf.format(),
        dmabuf.num_planes(),
        modifier,
    );

    Ok(AllocatedDmabuf {
        dmabuf,
        _bo: bo,
        _gbm: gbm,
    })
}
