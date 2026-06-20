//! DRM render node → gbm BO → Dmabuf.

use std::os::fd::{AsRawFd, OwnedFd};
use std::path::Path;

use compositor_support_bevy_core_fault_base::AllocError;
use compositor_developer_debug_instance_record::info;
use gbm::{BufferObjectFlags, Device as GbmDevice, Format as GbmFormat};
use smithay::backend::allocator::dmabuf::{Dmabuf, DmabufFlags};
use smithay::backend::allocator::{Buffer, Fourcc};

/// Opaque holder for an allocated buffer. Keeps gbm alive while the dmabuf
/// is in use. Drop order inside this struct: `dmabuf` first (releases fds
/// and any imports), then `_bo`, then `_gbm`.
pub struct AllocatedDmabuf {
    pub dmabuf: Dmabuf,
    _bo: gbm::BufferObject<()>,
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

/// Allocate a single ARGB8888 LINEAR dmabuf at the given size from the
/// default render node.
pub fn allocate_dmabuf(
    render_node: &str,
    width: u32,
    height: u32,
) -> Result<AllocatedDmabuf, AllocError> {
    allocate_dmabuf_on(Path::new(render_node), width, height)
}

/// Variant with an explicit render node path.
pub fn allocate_dmabuf_on(
    render_node: &Path,
    width: u32,
    height: u32,
) -> Result<AllocatedDmabuf, AllocError> {
    if width == 0 || height == 0 {
        return Err(AllocError::InvalidDimensions { width, height });
    }

    let drm_file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(render_node)
        .map_err(AllocError::OpenDrm)?;
    let drm_fd: OwnedFd = drm_file.into();

    info!("Opened DRM render node {} (fd={})", render_node.display(), drm_fd.as_raw_fd());

    let gbm = GbmDevice::new(drm_fd).map_err(AllocError::GbmInit)?;

    let bo = gbm
        .create_buffer_object::<()>(
            width,
            height,
            GbmFormat::Argb8888,
            BufferObjectFlags::RENDERING,
        )
        .map_err(AllocError::CreateBo)?;

    info!("Allocated gbm BO {}x{}, format=ARGB8888, modifier={:?}", width, height, bo.modifier());

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

    info!("Built Dmabuf: size={:?}, format={:?}, num_planes={}, modifier={:?}", dmabuf.size(), dmabuf.format(), dmabuf.num_planes(), modifier);

    Ok(AllocatedDmabuf { dmabuf, _bo: bo, _gbm: gbm })
}
