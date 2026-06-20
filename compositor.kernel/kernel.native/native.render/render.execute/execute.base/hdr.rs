//! HDR output signalling (M5 stage C): set the connector `Colorspace`
//! (BT.2020 RGB) + `HDR_OUTPUT_METADATA` (PQ infoframe) via a one-time raw DRM
//! atomic commit. smithay's `DrmCompositor` owns the per-frame page-flip commit
//! but exposes no colorspace/HDR API, so we set these *connector* properties
//! ourselves; they are sticky atomic state and persist across smithay's flips.
//!
//! SAFETY: every commit is preceded by a `TEST_ONLY` atomic commit. If the test
//! fails (malformed blob, unsupported property, wrong enum) we return an error
//! and the caller stays SDR — a bad blob can never blank the display.

use compositor_kernel_drm_edid_parse_base::parse::HdrInfo;
use smithay::backend::drm::DrmDeviceFd;
use smithay::reexports::drm::control::atomic::AtomicModeReq;
use smithay::reexports::drm::control::{
    connector, property, AtomicCommitFlags, Device as ControlDevice,
};

/// Kernel `struct hdr_metadata_infoframe` (CTA-861 static metadata type 1).
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct InfoframeXy {
    x: u16,
    y: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HdrMetadataInfoframe {
    /// `enum hdmi_eotf`: 0 SDR, 1 HDR gamma, 2 ST 2084 (PQ), 3 HLG.
    eotf: u8,
    /// Static metadata descriptor id (0 = type 1).
    metadata_type: u8,
    display_primaries: [InfoframeXy; 3],
    white_point: InfoframeXy,
    /// cd/m² (1-nit units).
    max_display_mastering_luminance: u16,
    /// 0.0001 cd/m² units.
    min_display_mastering_luminance: u16,
    /// cd/m².
    max_cll: u16,
    /// cd/m².
    max_fall: u16,
}

/// Kernel `struct hdr_output_metadata`. The blob the connector property takes.
#[repr(C)]
#[derive(Clone, Copy)]
struct HdrOutputMetadata {
    /// 0 = HDMI_STATIC_METADATA_TYPE1.
    metadata_type: u32,
    infoframe: HdrMetadataInfoframe,
}

/// EOTF code for SMPTE ST 2084 (PQ).
const EOTF_PQ: u8 = 2;
const EOTF_HLG: u8 = 3;

/// CIE xy → kernel 0.00002-unit fixed point (value = coord * 50000).
fn xy(coord: (f32, f32)) -> InfoframeXy {
    InfoframeXy {
        x: (coord.0 * 50000.0).round().clamp(0.0, 65535.0) as u16,
        y: (coord.1 * 50000.0).round().clamp(0.0, 65535.0) as u16,
    }
}

/// BT.2020 primaries (used when the EDID chromaticity is absent/zero).
const BT2020: ([(f32, f32); 3], (f32, f32)) = (
    [(0.708, 0.292), (0.170, 0.797), (0.131, 0.046)],
    (0.3127, 0.3290),
);

fn build_metadata(caps: &HdrInfo) -> HdrOutputMetadata {
    let p = &caps.primaries;
    // Use EDID primaries when present (non-zero), else fall back to BT.2020.
    let have_edid = p.red.0 > 0.0 && p.green.0 > 0.0 && p.blue.0 > 0.0 && p.white.0 > 0.0;
    let (prim, white) = if have_edid {
        ([p.red, p.green, p.blue], p.white)
    } else {
        BT2020
    };
    let max = caps.hdr.max_luminance.unwrap_or(0.0);
    let min = caps.hdr.min_luminance.unwrap_or(0.0);
    let fall = caps.hdr.max_frame_avg_luminance.unwrap_or(0.0);
    let eotf = if caps.hdr.eotf_pq { EOTF_PQ } else { EOTF_HLG };
    HdrOutputMetadata {
        metadata_type: 0,
        infoframe: HdrMetadataInfoframe {
            eotf,
            metadata_type: 0,
            display_primaries: [xy(prim[0]), xy(prim[1]), xy(prim[2])],
            white_point: xy(white),
            max_display_mastering_luminance: max.round().clamp(0.0, 65535.0) as u16,
            min_display_mastering_luminance: (min * 10000.0).round().clamp(0.0, 65535.0) as u16,
            max_cll: max.round().clamp(0.0, 65535.0) as u16,
            max_fall: fall.round().clamp(0.0, 65535.0) as u16,
        },
    }
}

/// Look up a connector property handle by name.
fn prop_handle(
    drm: &DrmDeviceFd,
    conn: connector::Handle,
    name: &str,
) -> Result<property::Handle, String> {
    let set = drm
        .get_properties(conn)
        .map_err(|e| format!("get_properties: {e}"))?;
    for h in set.as_props_and_values().0 {
        if let Ok(info) = drm.get_property(*h) {
            if info.name().to_str() == Ok(name) {
                return Ok(*h);
            }
        }
    }
    Err(format!("connector lacks property {name}"))
}

/// Raw value of the `Colorspace` enum entry named `BT2020_RGB`.
fn bt2020_colorspace_value(
    drm: &DrmDeviceFd,
    handle: property::Handle,
) -> Result<u64, String> {
    let info = drm
        .get_property(handle)
        .map_err(|e| format!("get_property(Colorspace): {e}"))?;
    if let property::ValueType::Enum(values) = info.value_type() {
        let (raws, enums) = values.values();
        for (raw, ev) in raws.iter().zip(enums.iter()) {
            if ev.name().to_str() == Ok("BT2020_RGB") {
                return Ok(*raw);
            }
        }
        return Err("Colorspace has no BT2020_RGB entry".into());
    }
    Err("Colorspace is not an enum property".into())
}

/// Apply BT.2020 + PQ HDR signalling to the connector. Returns Err (and leaves
/// the output SDR) if the kernel rejects the TEST commit.
pub fn signal_hdr(
    drm: &DrmDeviceFd,
    conn: connector::Handle,
    caps: &HdrInfo,
) -> Result<(), String> {
    let colorspace = prop_handle(drm, conn, "Colorspace")?;
    let hdr_meta = prop_handle(drm, conn, "HDR_OUTPUT_METADATA")?;
    let bt2020 = bt2020_colorspace_value(drm, colorspace)?;

    let metadata = build_metadata(caps);
    let blob = drm
        .create_property_blob(&metadata)
        .map_err(|e| format!("create_property_blob: {e}"))?;
    let blob_raw: u64 = blob.into();

    let build = || {
        let mut req = AtomicModeReq::new();
        req.add_property(conn, colorspace, property::Value::UnsignedRange(bt2020));
        req.add_property(conn, hdr_meta, property::Value::Blob(blob_raw));
        req
    };

    // Validate before committing — a bad blob fails here, never on screen.
    if let Err(e) = drm.atomic_commit(
        AtomicCommitFlags::TEST_ONLY | AtomicCommitFlags::ALLOW_MODESET,
        build(),
    ) {
        let _ = drm.destroy_property_blob(blob_raw);
        return Err(format!("HDR atomic TEST commit rejected: {e}"));
    }
    if let Err(e) = drm.atomic_commit(AtomicCommitFlags::ALLOW_MODESET, build()) {
        let _ = drm.destroy_property_blob(blob_raw);
        return Err(format!("HDR atomic commit failed: {e}"));
    }
    // Leak the blob for the session: the connector property references it while
    // active; we set it once and never replace it.
    Ok(())
}
