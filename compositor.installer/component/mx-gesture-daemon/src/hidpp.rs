//! Minimal HID++ 2.0 wire protocol.
//!
//! Reports we care about:
//!   - 0x10 short  (7 bytes total: 0x10, devidx, feat_idx<<4 | sw_id, 4 bytes params)
//!   - 0x11 long  (20 bytes total: 0x11, devidx, feat_idx<<4 | sw_id, 16 bytes params)
//!
//! For a device addressed directly (USB cable or Bluetooth), devidx is 0xFF.
//! For a device behind a Unifying/Bolt receiver, devidx is 1..=6.

use anyhow::{anyhow, bail, Context, Result};
use hidapi::HidDevice;
use std::time::{Duration, Instant};

pub const REPORT_SHORT: u8 = 0x10;
pub const REPORT_LONG: u8 = 0x11;

pub const SHORT_LEN: usize = 7;
pub const LONG_LEN: usize = 20;

/// Software ID. HID++ embeds this nibble in every request so responses can be
/// matched. Anything 1..=15 is fine; we use 5 (arbitrary, not 0).
pub const SW_ID: u8 = 0x5;

/// Direct addressing (no receiver between us and the device).
pub const DEVIDX_DIRECT: u8 = 0xFF;

/// Root feature is always at index 0.
pub const FEAT_ROOT: u8 = 0x00;

/// Feature IDs (the things you look up via root.getFeature).
pub const FID_ROOT: u16 = 0x0000;
pub const FID_FEATURE_SET: u16 = 0x0001;
pub const FID_DEVICE_NAME: u16 = 0x0005;
pub const FID_REPROG_CONTROLS_V4: u16 = 0x1B04;
pub const FID_WIRELESS_DEVICE_STATUS: u16 = 0x1D4B;

/// 0x1B04 (REPROG_CONTROLS_V4) function indices.
pub mod reprog {
    // Requests (what we send)
    pub const F_GET_COUNT: u8 = 0x0;
    pub const F_GET_CID_INFO: u8 = 0x1;
    pub const F_GET_REPORTING: u8 = 0x2;
    pub const F_SET_REPORTING: u8 = 0x3;

    // Event/notification function indices (what the device sends back as
    // notifications, in the high nibble of byte[3] with low nibble = 0).
    // These match Solaar's `hidpp20.py` REPROG_CONTROLS handling.
    pub const E_DIVERTED_BUTTONS: u8 = 0x0; // CIDs currently held (up to 4)
    pub const E_DIVERTED_RAW_XY: u8 = 0x1; // dx/dy while a diverted button is held
}

/// Known Control IDs (CIDs) — full list lives in Solaar's `special_keys.py`.
/// We only need a few.
pub mod cid {
    pub const LEFT_BUTTON: u16 = 0x50;
    pub const RIGHT_BUTTON: u16 = 0x51;
    pub const MIDDLE_BUTTON: u16 = 0x52;
    pub const BACK_BUTTON: u16 = 0x53;
    pub const FORWARD_BUTTON: u16 = 0x56;
    pub const MOUSE_GESTURE_BUTTON: u16 = 0xC3;
    pub const SMART_SHIFT: u16 = 0xC4;
}

pub fn cid_name(cid: u16) -> &'static str {
    match cid {
        cid::LEFT_BUTTON => "Left Button",
        cid::RIGHT_BUTTON => "Right Button",
        cid::MIDDLE_BUTTON => "Middle Button",
        cid::BACK_BUTTON => "Back Button",
        cid::FORWARD_BUTTON => "Forward Button",
        cid::MOUSE_GESTURE_BUTTON => "Mouse Gesture Button",
        cid::SMART_SHIFT => "Smart Shift",
        _ => "Unknown",
    }
}

/// Build a long-format HID++ request (20 bytes).
fn build_long(devidx: u8, feat_idx: u8, func: u8, params: &[u8]) -> [u8; LONG_LEN] {
    let mut buf = [0u8; LONG_LEN];
    buf[0] = REPORT_LONG;
    buf[1] = devidx;
    buf[2] = feat_idx;
    // function index in high nibble, software id in low nibble
    buf[3] = (func << 4) | (SW_ID & 0x0F);
    let n = params.len().min(LONG_LEN - 4);
    buf[4..4 + n].copy_from_slice(&params[..n]);
    buf
}

/// Send a HID++ long request and wait for the matching response.
///
/// Responses can arrive in three shapes:
/// 1. Normal HID++ 2.0 long response (0x11). devidx matches what we sent,
///    feature index and func|swid byte match.
/// 2. HID++ 2.0 error (long, 0x11). feature index in the response is 0xFF;
///    the original feature index is in byte 4 and the error code in byte 5.
/// 3. HID++ 1.0 error (short, 0x10). sub_id is 0x8F. Notably the devidx
///    here can be the *resolved* slot if we asked the receiver with 0xFF.
pub fn request_long(
    dev: &HidDevice,
    devidx: u8,
    feat_idx: u8,
    func: u8,
    params: &[u8],
) -> Result<[u8; LONG_LEN]> {
    let req = build_long(devidx, feat_idx, func, params);
    log::trace!("TX {:02X?}", &req[..]);
    dev.write(&req).context("write HID++ request")?;

    let deadline = Instant::now() + Duration::from_millis(600);
    let mut buf = [0u8; 64];
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let to = remaining.as_millis().min(150) as i32;
        let n = dev
            .read_timeout(&mut buf, to)
            .context("read HID++ response")?;
        if n == 0 {
            continue;
        }
        log::trace!("RX {:02X?}", &buf[..n]);

        // HID++ 1.0 error, short report:
        //   0x10 devidx 0x8F sub_id register err_code 0x00
        // We don't strictly check devidx because a receiver may rewrite it
        // to the resolved slot. We do require sub_id == 0x8F and that the
        // sub_id field is at the expected offset.
        if buf[0] == REPORT_SHORT && n >= SHORT_LEN && buf[2] == 0x8F {
            let err = buf[5];
            bail!(
                "HID++ 1.0 error from devidx 0x{:02X}: code 0x{:02X} ({})",
                buf[1],
                err,
                hidpp10_error_name(err)
            );
        }

        // Same error but in long format (some firmwares do this).
        if buf[0] == REPORT_LONG && n >= LONG_LEN && buf[2] == 0x8F {
            let err = buf[5];
            bail!(
                "HID++ 1.0 error (long) from devidx 0x{:02X}: code 0x{:02X} ({})",
                buf[1],
                err,
                hidpp10_error_name(err)
            );
        }

        // HID++ 2.0 error: feature index 0xFF in response, original feat
        // index in byte 4, error code in byte 5.
        if buf[0] == REPORT_LONG
            && n >= LONG_LEN
            && buf[1] == devidx
            && buf[2] == 0xFF
            && buf[4] == feat_idx
        {
            let err = buf[5];
            bail!(
                "HID++ 2.0 error: feature 0x{:02X} func 0x{:X} err 0x{:02X} ({})",
                feat_idx,
                func,
                err,
                hidpp20_error_name(err)
            );
        }

        // Normal long response.
        if buf[0] == REPORT_LONG
            && n >= LONG_LEN
            && buf[1] == devidx
            && buf[2] == feat_idx
            && buf[3] == ((func << 4) | SW_ID)
        {
            let mut out = [0u8; LONG_LEN];
            out.copy_from_slice(&buf[..LONG_LEN]);
            return Ok(out);
        }

        // Anything else is an unrelated notification — keep waiting.
    }
    Err(anyhow!("HID++ request timed out"))
}

fn hidpp10_error_name(code: u8) -> &'static str {
    match code {
        0x00 => "SUCCESS",
        0x01 => "INVALID_SUBID",
        0x02 => "INVALID_ADDRESS",
        0x03 => "INVALID_VALUE",
        0x04 => "CONNECT_FAIL",
        0x05 => "TOO_MANY_DEVICES",
        0x06 => "ALREADY_EXISTS",
        0x07 => "BUSY",
        0x08 => "UNKNOWN_DEVICE",
        0x09 => "RESOURCE_ERROR / device asleep or unpaired",
        0x0A => "REQUEST_UNAVAILABLE",
        0x0B => "INVALID_PARAM_VALUE",
        0x0C => "WRONG_PIN_CODE",
        _ => "unknown",
    }
}

fn hidpp20_error_name(code: u8) -> &'static str {
    match code {
        0x00 => "NO_ERROR",
        0x01 => "UNKNOWN",
        0x02 => "INVALID_ARGS",
        0x03 => "OUT_OF_RANGE",
        0x04 => "HW_ERROR",
        0x05 => "LOGITECH_INTERNAL",
        0x06 => "INVALID_FEATURE_INDEX",
        0x07 => "INVALID_FUNCTION_ID",
        0x08 => "BUSY",
        0x09 => "UNSUPPORTED",
        _ => "unknown",
    }
}

/// Quick liveness check. Sends a short root.getProtocolVersion ping and
/// considers the node "alive" if we get ANY HID++ reply — a real response,
/// a HID++ 1.0 error, or a HID++ 2.0 error. Times out fast (150ms) so
/// dead/unrelated hidraw nodes don't slow discovery to a crawl.
pub fn ping(dev: &HidDevice, devidx: u8) -> bool {
    let req = build_long(devidx, FEAT_ROOT, 0x1, &[0, 0, 0xCA]); // ping byte
    if dev.write(&req).is_err() {
        return false;
    }
    let deadline = Instant::now() + Duration::from_millis(150);
    let mut buf = [0u8; 64];
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let to = remaining.as_millis().min(50) as i32;
        match dev.read_timeout(&mut buf, to) {
            Ok(n) if n > 0 => {
                // Any short or long HID++ frame we recognise = alive.
                if buf[0] == REPORT_SHORT || buf[0] == REPORT_LONG {
                    return true;
                }
            }
            Ok(_) => continue,
            Err(_) => return false,
        }
    }
    false
}

/// Resolve a feature ID to its per-device feature index via the root feature.
///
/// Returns:
///   Ok(Some(idx))  feature is present at index `idx`
///   Ok(None)       device responded but does not implement this feature
///                  (feat_idx == 0 in response)
///   Err(...)       transport error, timeout, or HID++ error (e.g. slot empty)
pub fn get_feature_index(dev: &HidDevice, devidx: u8, feature_id: u16) -> Result<Option<u8>> {
    let resp = request_long(
        dev,
        devidx,
        FEAT_ROOT,
        0x0, // root.getFeature
        &[(feature_id >> 8) as u8, feature_id as u8],
    )?;
    let idx = resp[4];
    if idx == 0 {
        Ok(None)
    } else {
        Ok(Some(idx))
    }
}

/// 0x1B04 getCidInfo — returns the 16-byte info block. We only parse the
/// "divertable" capability flag.
pub fn get_cid_info(
    dev: &HidDevice,
    devidx: u8,
    feat_idx: u8,
    index: u8,
) -> Result<CidInfo> {
    let resp = request_long(dev, devidx, feat_idx, reprog::F_GET_CID_INFO, &[index])?;
    // Layout (V4+):
    //   [4..6]  CID (u16, big-endian)
    //   [6..8]  task ID (u16)
    //   [8]     flags1 (bit4 = divertable)
    //   ...
    let cid = u16::from_be_bytes([resp[4], resp[5]]);
    let flags1 = resp[8];
    let divertable = (flags1 & 0x10) != 0;
    Ok(CidInfo { cid, divertable })
}

#[derive(Debug, Clone, Copy)]
pub struct CidInfo {
    pub cid: u16,
    pub divertable: bool,
}

/// 0x1B04 setReporting. Bits we use in the reporting flags byte:
///   bit 0  divert change valid
///   bit 1  diverted (0 = regular, 1 = diverted)
///   bit 2  persist change valid
///   bit 3  persistent (only meaningful if persist-valid)
///   bit 4  raw XY change valid
///   bit 5  raw XY (1 = report raw XY while held — gesture button mode)
pub fn set_reporting(
    dev: &HidDevice,
    devidx: u8,
    feat_idx: u8,
    cid: u16,
    diverted: bool,
    raw_xy: bool,
) -> Result<()> {
    let mut flags = 0u8;
    // mark the bits we are changing as valid
    flags |= 0b0000_0001; // divert valid
    if diverted {
        flags |= 0b0000_0010;
    }
    flags |= 0b0001_0000; // raw-xy valid
    if raw_xy {
        flags |= 0b0010_0000;
    }
    let params = [(cid >> 8) as u8, cid as u8, flags, 0, 0];
    let _ = request_long(dev, devidx, feat_idx, reprog::F_SET_REPORTING, &params)?;
    Ok(())
}

/// Extract the gesture direction from a diverted-button "raw XY" notification.
/// Sign of dx/dy and which axis dominates → 4-way direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::Up => "up",
            Direction::Down => "down",
            Direction::Left => "left",
            Direction::Right => "right",
        }
    }
}

/// Parse dx/dy from a "diverted raw XY" notification payload (the 16 bytes
/// after the 4-byte header). Layout for E_DIVERTED_RAW_XY:
///   payload[0..2]   dx (i16 BE)
///   payload[2..4]   dy (i16 BE)
pub fn parse_raw_xy(payload: &[u8]) -> Option<(i16, i16)> {
    if payload.len() < 4 {
        return None;
    }
    let dx = i16::from_be_bytes([payload[0], payload[1]]);
    let dy = i16::from_be_bytes([payload[2], payload[3]]);
    Some((dx, dy))
}

/// Parse the diverted-buttons notification payload — up to 4 currently-held
/// CIDs, big-endian u16 each. Trailing zeros mean "no more buttons held".
pub fn parse_diverted_buttons(payload: &[u8]) -> [u16; 4] {
    let mut out = [0u16; 4];
    for (i, slot) in out.iter_mut().enumerate() {
        let off = i * 2;
        if payload.len() >= off + 2 {
            *slot = u16::from_be_bytes([payload[off], payload[off + 1]]);
        }
    }
    out
}
