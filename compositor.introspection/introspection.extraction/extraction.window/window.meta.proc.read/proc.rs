use compositor_introspection_extraction_window_meta_types::env::ENV_ALLOWLIST;
use compositor_introspection_extraction_window_meta_types::types::Meta;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Read everything we want from `/proc/<pid>/...` into a `Meta`.
///
/// Returns None only if the process has gone away or /proc isn't readable.
/// Individual sub-reads that fail just leave their field as None.
pub fn extract_meta_for_pid(pid: u32) -> Option<Meta> {
    let proc_dir = format!("/proc/{pid}");

    // Probe that the directory exists at all; otherwise the process is gone.
    if fs::metadata(&proc_dir).is_err() {
        return None;
    }

    let mut info = Meta::default();
    info.pid = Some(pid);
    info.comm = fs::read_to_string(format!("{proc_dir}/comm"))
        .ok()
        .map(|s| s.trim().to_string());
    info.exe = fs::read_link(format!("{proc_dir}/exe")).ok();
    info.cwd = fs::read_link(format!("{proc_dir}/cwd")).ok();
    info.cgroup = fs::read_to_string(format!("{proc_dir}/cgroup")).ok();
    info.cmdline = fs::read(format!("{proc_dir}/cmdline")).ok().map(|bytes| {
        bytes
            .split(|b| *b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect()
    });
    info.selected_env = read_filtered_env(&proc_dir);
    Some(info)
}

fn read_filtered_env(proc_dir: &str) -> Option<HashMap<String, String>> {
    let bytes = fs::read(format!("{proc_dir}/environ")).ok()?;
    let mut out = HashMap::new();
    for entry in bytes.split(|b| *b == 0) {
        if entry.is_empty() {
            continue;
        }
        let s = String::from_utf8_lossy(entry);
        if let Some(eq) = s.find('=') {
            let k = &s[..eq];
            let v = &s[eq + 1..];
            if ENV_ALLOWLIST.contains(&k) {
                out.insert(k.to_string(), v.to_string());
            }
        }
    }
    Some(out)
}

/// `/proc/<pid>` as a path. (Sandbox interpretation lives in the hint layer;
/// this module just yields the raw cgroup string in Meta.cgroup.)
pub fn proc_dir(pid: u32) -> PathBuf {
    PathBuf::from(format!("/proc/{pid}"))
}
