//! Spawn apps via `systemd-run --user`: each launch becomes a transient
//! `.service` unit under the user's systemd manager, so apps are
//! cgroup-isolated, journald-logged, and decoupled from compositor crashes.

use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use compositor_introspection_launchplan_plan_exec_opts::exec::{ManagedSpawn, SystemdRunOpts};
use compositor_introspection_launchplan_plan_exec_pid::exec::{poll_main_pid, resolve_program};

/// Translate a configured Command into a systemd-run --user invocation,
/// execute it (blocking until systemd-run returns — typically a few ms),
/// then poll for the unit's MainPID up to opts.pid_poll_timeout.
pub fn wrap_and_execute(original: &Command, opts: &SystemdRunOpts) -> io::Result<ManagedSpawn> {
    let mut sr = Command::new("systemd-run");

    sr.arg("--user");
    sr.arg("--quiet");
    sr.arg("--collect");
    sr.arg("--no-ask-password");
    sr.arg(format!("--unit={}", opts.unit));

    if let Some(desc) = &opts.description {
        sr.arg(format!("--description={desc}"));
    } else {
        let basename = std::path::Path::new(original.get_program())
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("unknown");
        sr.arg(format!("--description=launcher: {basename}"));
    }

    // Env: --setenv per pair. Skip env_remove entries (None values).
    for (k, maybe_v) in original.get_envs() {
        if let Some(v) = maybe_v {
            sr.arg(format!("--setenv={}={}", k.to_string_lossy(), v.to_string_lossy()));
        }
    }

    // current_dir -> WorkingDirectory property (must be absolute for .service units).
    if let Some(wd) = original.get_current_dir() {
        let abs = if wd.is_absolute() {
            wd.to_path_buf()
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(wd))
                .unwrap_or_else(|_| wd.to_path_buf())
        };
        sr.arg(format!("--property=WorkingDirectory={}", abs.display()));
    }

    if let Some(t) = &opts.part_of {
        sr.arg(format!("--property=PartOf={t}"));
        sr.arg(format!("--property=After={t}"));
    }
    if let Some(t) = opts.timeout_stop_sec {
        sr.arg(format!("--property=TimeoutStopSec={t}"));
    }
    if let Some(n) = opts.tasks_max {
        sr.arg(format!("--property=TasksMax={n}"));
    }
    if let Some(m) = &opts.memory_max {
        sr.arg(format!("--property=MemoryMax={m}"));
    }
    for (k, v) in &opts.extra_properties {
        sr.arg(format!("--property={k}={v}"));
    }

    sr.arg("--");

    // Resolve bare program names via $PATH; systemd needs absolute paths.
    let prog = original.get_program();
    let abs_prog = resolve_program(prog).unwrap_or_else(|| PathBuf::from(prog));
    sr.arg(abs_prog);
    for a in original.get_args() {
        sr.arg(a);
    }

    sr.stdin(Stdio::null());
    sr.stdout(Stdio::null());
    sr.stderr(Stdio::null());

    let status = sr.status()?;
    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("systemd-run --user exited with {status} (unit={})", opts.unit),
        ));
    }

    // Poll for MainPID; may yield None within budget (caller opted in).
    let pid = poll_main_pid(&opts.unit, opts.pid_poll_timeout, opts.pid_poll_interval);

    Ok(ManagedSpawn { unit: opts.unit.clone(), pid })
}
