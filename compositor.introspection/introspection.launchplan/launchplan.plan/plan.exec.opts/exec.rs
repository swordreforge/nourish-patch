//! Options/result types for systemd-run wrapped launches.

use std::time::Duration;

pub struct SystemdRunOpts {
    /// Required: stable unit name (without .service). Needed for MainPID lookup.
    pub unit: String,
    pub description: Option<String>,
    /// Default "graphical-session.target". None to opt out.
    pub part_of: Option<String>,
    pub timeout_stop_sec: Option<u32>,
    pub tasks_max: Option<u64>,
    pub memory_max: Option<String>,
    pub extra_properties: Vec<(String, String)>,
    /// Max time to wait for the unit to report a non-zero MainPID.
    pub pid_poll_timeout: Duration,
    /// Polling interval. 25ms is a good balance for ~1s budget.
    pub pid_poll_interval: Duration,
}

impl SystemdRunOpts {
    pub fn new(unit: impl Into<String>) -> Self {
        Self {
            unit: unit.into(),
            description: None,
            part_of: Some("graphical-session.target".into()),
            timeout_stop_sec: Some(5),
            tasks_max: None,
            memory_max: None,
            extra_properties: Vec::new(),
            pid_poll_timeout: Duration::from_secs(1),
            pid_poll_interval: Duration::from_millis(25),
        }
    }

    pub fn new_detach(unit: impl Into<String>) -> Self {
        Self {
            unit: unit.into(),
            description: None,
            part_of: Some("graphical-session.target".into()),
            timeout_stop_sec: Some(5),
            tasks_max: None,
            memory_max: None,
            extra_properties: Vec::new(),
            pid_poll_timeout: Duration::from_secs(0),
            pid_poll_interval: Duration::from_millis(25),
        }
    }
}

/// Outcome of wrap_and_execute.
#[derive(Debug)]
pub struct ManagedSpawn {
    pub unit: String,
    /// MainPID of the launched process. `None` means systemd-run succeeded but
    /// we couldn't read a non-zero MainPID within pid_poll_timeout. Caller
    /// decided this is acceptable.
    pub pid: Option<u32>,
}
