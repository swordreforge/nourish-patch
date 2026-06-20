//! Computes the rim-facing `DisplaySnapshot` (external present? internal panel
//! active?) from a live connector scan. The kernel owns the DRM device; the rim
//! reads only this primitive summary via the lid driver token.

use compositor_kernel_drm_connector_kind_base::kind::{classify, ConnectorKind};
use compositor_orchestration_driver_lid_base::base::DisplaySnapshot;
use smithay::backend::drm::DrmDevice;
use smithay::reexports::drm::control::connector;

/// Scan all connectors on `drm` and summarize for the lid policy. `active` is the
/// connector currently driving the output (its kind decides `internal_active`).
pub fn compute(drm: &DrmDevice, active: connector::Handle) -> DisplaySnapshot {
    let res = compositor_kernel_drm_connector_scan_base::scan::resources(drm);
    let infos = compositor_kernel_drm_connector_scan_base::scan::connectors(drm, &res);

    let mut snapshot = DisplaySnapshot::default();
    for info in &infos {
        let kind = classify(info);
        let connected = info.state() == connector::State::Connected;
        if connected && kind == ConnectorKind::External {
            snapshot.external_present = true;
        }
        if info.handle() == active && kind == ConnectorKind::InternalPanel {
            snapshot.internal_active = true;
        }
    }
    snapshot
}
