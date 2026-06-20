//! MainPID polling and $PATH resolution for systemd-managed launches.

use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Blocking poll for `MainPID` of a user-scoped transient unit.
///
/// `systemctl show -p MainPID --value` prints the PID followed by a newline,
/// or "0" if the unit hasn't started yet or has already exited.
///
/// Returns the first non-zero PID observed, or None if the timeout is reached
/// with only zeros (or systemctl errors).
pub fn poll_main_pid(unit: &str, timeout: Duration, interval: Duration) -> Option<u32> {
    if timeout.is_zero() {
        return None;
    }

    let deadline = Instant::now() + timeout;
    let unit_full = format!("{unit}.service");

    loop {
        match Command::new("systemctl")
            .args(["--user", "show", "-p", "MainPID", "--value", &unit_full])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
        {
            Ok(out) if out.status.success() => {
                let s = String::from_utf8_lossy(&out.stdout);
                if let Ok(pid) = s.trim().parse::<u32>() {
                    if pid != 0 {
                        return Some(pid);
                    }
                }
            }
            // systemctl failed (unit not found yet, dbus hiccup, etc.). Treat
            // like "0" and retry until deadline.
            _ => {}
        }

        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(interval);
    }
}

/// Resolve bare program names via $PATH; systemd needs absolute paths.
pub fn resolve_program(prog: &OsStr) -> Option<PathBuf> {
    let p = std::path::Path::new(prog);
    if p.is_absolute() {
        return Some(p.to_path_buf());
    }
    let path_env = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_env) {
        let cand = dir.join(p);
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}
