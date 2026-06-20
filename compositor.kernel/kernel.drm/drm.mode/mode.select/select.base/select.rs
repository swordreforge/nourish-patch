//! Mode selection policy (ex wire.rs): area -> refresh -> PREFERRED,
//! lexicographic. Consumes preference values when a profile requests an
//! advertised mode; otherwise unchanged default policy.

use compositor_kernel_graphic_preference_output_profile::profile::{ModeRequest, OutputProfile};
use smithay::reexports::drm::control::{connector, Mode as DrmMode, ModeTypeFlags};

/// Default policy, byte-for-byte the original ordering.
pub fn select_default(info: &connector::Info) -> Option<DrmMode> {
    info.modes()
        .iter()
        .max_by_key(|m| {
            let (w, h) = m.size();
            let area = (w as u64) * (h as u64);
            let refresh = m.vrefresh();
            let is_preferred = m.mode_type().contains(ModeTypeFlags::PREFERRED);
            (area, refresh, is_preferred)
        })
        .copied()
}

/// Preference-aware selection: an `Advertised` request narrows the list; any
/// synthesis request is NOT handled here (that is `mode.synthesize`, Law 7).
pub fn select(info: &connector::Info, profile: Option<&OutputProfile>) -> Option<DrmMode> {
    if let Some(OutputProfile { mode: Some(ModeRequest::Advertised { width, height, refresh_mhz }), .. }) = profile {
        let hit = info.modes().iter().find(|m| {
            let (w, h) = m.size();
            w == *width && h == *height && m.vrefresh() * 1000 == *refresh_mhz
        });
        if let Some(m) = hit {
            return Some(*m);
        }
        warn!("requested advertised mode not found; falling back to default policy");
    }
    select_default(info)
}

pub fn log_selected(mode: &DrmMode) {
    info!(
        "selected mode: {}x{} @ {}Hz, type: {:?}",
        mode.size().0,
        mode.size().1,
        mode.vrefresh(),
        mode.mode_type(),
    );
}
