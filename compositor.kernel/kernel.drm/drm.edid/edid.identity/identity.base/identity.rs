//! Stable monitor identity for preferences + PhysicalProperties
//! (replacing the hardcoded "Native"/"Monitor"/"Unknown").

use compositor_kernel_drm_edid_parse_base::parse::ParsedEdid;

#[derive(Debug, Clone)]
pub struct MonitorIdentity {
    pub make: String,
    pub model: String,
    pub serial: String,
}

impl MonitorIdentity {
    /// The placeholder identity the original code hardcoded; used when no
    /// EDID is readable so behavior is unchanged on that path.
    pub fn unknown() -> Self {
        Self {
            make: "Native".into(),
            model: "Monitor".into(),
            serial: "Unknown".into(),
        }
    }

    /// The stable key preferences are matched against.
    pub fn key(&self) -> String {
        format!("{} {} {}", self.make, self.model, self.serial)
    }
}

pub fn identity(parsed: Option<&ParsedEdid>) -> MonitorIdentity {
    match parsed {
        None => MonitorIdentity::unknown(),
        Some(p) => MonitorIdentity {
            make: p.manufacturer.clone(),
            model: p
                .display_name
                .clone()
                .unwrap_or_else(|| format!("{:04X}", p.product_code)),
            serial: if p.serial == 0 {
                "Unknown".into()
            } else {
                p.serial.to_string()
            },
        },
    }
}
