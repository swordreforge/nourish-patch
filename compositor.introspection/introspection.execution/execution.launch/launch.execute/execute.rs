//! `execute()` — the one place a child is actually spawned.

use std::process::{Command, Stdio};

use compositor_introspection_execution_launch_policy::policy::{LaunchBackend, LAUNCH_BACKEND};
use compositor_introspection_execution_launch_scope::scope::adopt_into_scope;
use compositor_introspection_execution_launch_types::types::{LaunchOutcome, LaunchRequest};

/// Spawn `req` and return its outcome. The PID is always `Child::id()` — every
/// backend self-spawns, so it is available synchronously and never polled out
/// of systemd. `SystemdScope` additionally adopts the live PID into a transient
/// scope (best-effort). The reaper (gated on the backend, wired in at startup)
/// owns reaping; we must never `wait` on the child here.
pub fn execute(req: &LaunchRequest) -> LaunchOutcome {
    let mut argv = req.argv.iter();
    let Some(program) = argv.next() else {
        return fail(req, "launch request has no program".into());
    };

    let mut cmd = Command::new(program);
    cmd.args(argv);
    for (k, v) in &req.env {
        cmd.env(k, v);
    }
    if let Some(dir) = &req.working_dir {
        cmd.current_dir(dir);
    }
    // Matches the historical path: children inherit the compositor's stdio.
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let pid = match cmd.spawn() {
        Ok(child) => {
            let pid = child.id();
            // std's Child does not wait on drop, so dropping here cannot race
            // the SIGCHLD reaper; it merely releases our handle.
            drop(child);
            pid
        }
        Err(e) => return fail(req, format!("spawn failed: {e}")),
    };

    if matches!(LAUNCH_BACKEND, LaunchBackend::SystemdScope) {
        if let Err(e) = adopt_into_scope(pid, &req.unit) {
            warn!("scope adoption failed for pid {pid} ({}): {e}", req.unit);
        }
    }

    LaunchOutcome { correlation: req.correlation, token: req.token.clone(), pid: Some(pid), result: Ok(()) }
}

fn fail(req: &LaunchRequest, reason: String) -> LaunchOutcome {
    warn!("launch failed: {reason}");
    LaunchOutcome { correlation: req.correlation, token: req.token.clone(), pid: None, result: Err(reason) }
}
