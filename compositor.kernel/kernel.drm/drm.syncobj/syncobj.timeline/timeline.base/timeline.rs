//! Timeline points: signal / wait / query on a device's timeline syncobjs.
//! Vulkan timeline semaphores (Phase 4 Step 2) bridge to exactly these calls.

use smithay::backend::drm::DrmDeviceFd;
use smithay::reexports::drm::control::{syncobj, Device as ControlDevice};

#[derive(Debug, Clone, Copy)]
pub struct TimelinePoint {
    pub handle: syncobj::Handle,
    pub point: u64,
}

pub fn signal(device: &DrmDeviceFd, tp: TimelinePoint) -> std::io::Result<()> {
    device.syncobj_timeline_signal(&[tp.handle], &[tp.point])
}

pub fn wait(device: &DrmDeviceFd, tp: TimelinePoint, timeout_nsec: i64) -> std::io::Result<()> {
    device
        .syncobj_timeline_wait(&[tp.handle], &[tp.point], timeout_nsec, false, false, false)
        .map(|_| ())
}

pub fn query_signalled(device: &DrmDeviceFd, handle: syncobj::Handle) -> std::io::Result<u64> {
    let mut points = [0u64];
    device.syncobj_timeline_query(&[handle], &mut points, false)?;
    Ok(points[0])
}
