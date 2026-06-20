//! Per-surface HDR color tag, shared between the `wp_color_management_v1`
//! protocol (writer, in the support layer) and the renderer (reader, in the
//! backend). A small leaf type stored in the `wl_surface`'s state so neither
//! layer has to depend on the other.

use smithay::wayland::compositor::SurfaceData;
use std::sync::Mutex;

/// A surface's declared content color, in renderer-friendly terms.
#[derive(Debug, Clone, Copy, Default)]
pub struct SurfaceHdr {
    /// Source transfer function: 0 sRGB, 1 PQ (ST 2084), 2 HLG, 3 linear.
    pub transfer: u8,
    /// True when the content is already in an HDR encoding (PQ/HLG) and should
    /// be passed through rather than SDR→PQ converted.
    pub is_hdr: bool,
}

type Cell = Mutex<Option<SurfaceHdr>>;

/// Record (or clear) a surface's color tag.
pub fn set(data: &SurfaceData, hdr: Option<SurfaceHdr>) {
    data.data_map.insert_if_missing(Cell::default);
    *data.data_map.get::<Cell>().unwrap().lock().unwrap() = hdr;
}

/// Read a surface's color tag, if set.
pub fn get(data: &SurfaceData) -> Option<SurfaceHdr> {
    data.data_map.get::<Cell>().and_then(|c| *c.lock().unwrap())
}
