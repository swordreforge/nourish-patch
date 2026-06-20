//! Device discovery.
//!
//! Logitech receivers (Unifying, Bolt, Lightspeed, Nano) host up to 6 paired
//! devices, each addressed via a "device index" 1..=6. A device connected
//! directly over USB cable or Bluetooth is addressed with device index 0xFF.
//!
//! On Linux/hidraw, a single Logitech receiver typically registers multiple
//! /dev/hidrawN entries (one per HID application collection in the report
//! descriptor: keyboard, mouse, consumer, HID++ short, HID++ long, ...).
//! Only one of those nodes accepts HID++ 0x10/0x11 reports. We can't always
//! tell which one from `usage_page` alone (some kernels return 0), so we
//! attempt a HID++ ping on every unique path and keep the ones that respond.

use anyhow::{anyhow, Context, Result};
use hidapi::{HidApi, HidDevice};
use std::collections::BTreeSet;

use crate::hidpp;

pub const LOGITECH_VID: u16 = 0x046D;

/// One physical hidraw node that speaks HID++.
#[derive(Debug, Clone)]
pub struct HidppPath {
    pub path: String,
    pub vid: u16,
    pub pid: u16,
    pub product: String,
}

/// A logical Logitech device: a particular slot reachable via a particular
/// hidraw path, with whatever name we managed to read for it.
#[derive(Debug, Clone)]
pub struct Device {
    pub path: HidppPath,
    /// Device index. 0xFF = directly attached. 1..=6 = receiver slot.
    pub devidx: u8,
    /// Friendly name as read from feature 0x0005 if available.
    pub name: String,
}

/// Return one entry per unique /dev/hidrawN that belongs to a Logitech device.
/// We dedupe on path so a receiver doesn't appear 5 times.
pub fn list_hidraw_paths(api: &HidApi) -> Vec<HidppPath> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out: Vec<HidppPath> = Vec::new();
    for d in api.device_list() {
        if d.vendor_id() != LOGITECH_VID {
            continue;
        }
        let path_str = d.path().to_string_lossy().into_owned();
        if !seen.insert(path_str.clone()) {
            continue;
        }
        out.push(HidppPath {
            path: path_str,
            vid: d.vendor_id(),
            pid: d.product_id(),
            product: d
                .product_string()
                .unwrap_or("(unknown)")
                .to_string(),
        });
    }
    out
}

/// Open a hidraw node by path.
pub fn open(api: &HidApi, path: &str) -> Result<HidDevice> {
    let cpath = std::ffi::CString::new(path).context("invalid device path")?;
    api.open_path(&cpath).context("open_path failed")
}

/// Try to read a device's friendly name via feature 0x0005 DEVICE_NAME.
/// Returns None if the device doesn't implement the feature or doesn't reply.
fn probe_device_name(dev: &HidDevice, devidx: u8) -> Option<String> {
    let feat_idx = hidpp::get_feature_index(dev, devidx, hidpp::FID_DEVICE_NAME).ok()??;

    // function 0 of DEVICE_NAME: getCount → byte 4 = total name length
    let resp = hidpp::request_long(dev, devidx, feat_idx, 0x0, &[]).ok()?;
    let total = resp[4] as usize;
    if total == 0 || total > 64 {
        return None;
    }

    // function 1: getDeviceName(offset). Returns up to 16 ASCII bytes per call.
    let mut buf: Vec<u8> = Vec::with_capacity(total);
    let mut offset: usize = 0;
    while buf.len() < total {
        let resp = hidpp::request_long(dev, devidx, feat_idx, 0x1, &[offset as u8]).ok()?;
        // The payload is in resp[4..]. Strip trailing zeros for the last chunk.
        let chunk = &resp[4..hidpp::LONG_LEN];
        let want = (total - buf.len()).min(chunk.len());
        for &b in &chunk[..want] {
            if b == 0 {
                break;
            }
            buf.push(b);
        }
        // Safety: don't loop forever if the device returns nothing useful.
        let new_offset = buf.len();
        if new_offset == offset {
            break;
        }
        offset = new_offset;
    }
    if buf.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&buf).trim().to_string())
    }
}

/// Probe a single hidraw path: try direct (0xFF) and each receiver slot (1..=6).
/// Return every responding device.
///
/// To keep probing snappy across many hidraw nodes, we use a short-timeout
/// ping first on 0xFF. If we get *any* HID++ response (success or error),
/// this node speaks HID++, and we proceed to enumerate slots. If nothing
/// comes back, we move on.
pub fn probe_path(api: &HidApi, p: &HidppPath) -> Vec<Device> {
    let mut out = Vec::new();
    let dev = match open(api, &p.path) {
        Ok(d) => d,
        Err(e) => {
            log::debug!("open {} failed: {}", p.path, e);
            return out;
        }
    };

    // Quick liveness check on 0xFF. Whether the response is a real device
    // ("here I am") or a HID++ 1.0 error (a receiver saying "no device at
    // slot FF"), we learn the node speaks HID++.
    if !hidpp::ping(&dev, 0xFF) {
        log::trace!("{} did not respond to HID++ ping; skipping", p.path);
        return out;
    }

    // Probe candidate device indexes.
    for &idx in &[0xFFu8, 1, 2, 3, 4, 5, 6] {
        match hidpp::get_feature_index(&dev, idx, hidpp::FID_ROOT) {
            Ok(_) => {
                let name = probe_device_name(&dev, idx).unwrap_or_else(|| {
                    if idx == 0xFF {
                        p.product.clone()
                    } else {
                        format!("Logitech device @ {} slot {}", p.product, idx)
                    }
                });
                log::debug!("found {} on {} idx 0x{:02X}", name, p.path, idx);
                out.push(Device {
                    path: p.clone(),
                    devidx: idx,
                    name,
                });
            }
            Err(e) => {
                log::trace!("{} idx 0x{:02X} no device: {}", p.path, idx, e);
            }
        }
    }
    out
}

/// Enumerate every Logitech device reachable on this machine.
pub fn discover_all(api: &HidApi) -> Vec<Device> {
    let paths = list_hidraw_paths(api);
    let mut out = Vec::new();
    for p in paths {
        out.extend(probe_path(api, &p));
    }
    out
}

/// Find a device whose name contains `hint` (case-insensitive).
pub fn find_by_hint<'a>(devs: &'a [Device], hint: &str) -> Option<&'a Device> {
    let h = hint.to_lowercase();
    devs.iter().find(|d| d.name.to_lowercase().contains(&h))
}

/// Find a device by its hidraw-node hex PID (e.g. "B034" for MX Master 3S via BT,
/// "405B" for a Bolt receiver, etc.). When you only have a receiver PID, this
/// will find the *first* slot under that receiver.
pub fn find_by_pid<'a>(devs: &'a [Device], pid_hex: &str) -> Result<Option<&'a Device>> {
    let pid = u16::from_str_radix(pid_hex.trim_start_matches("0x"), 16)
        .map_err(|_| anyhow!("invalid PID: {}", pid_hex))?;
    Ok(devs.iter().find(|d| d.path.pid == pid))
}
