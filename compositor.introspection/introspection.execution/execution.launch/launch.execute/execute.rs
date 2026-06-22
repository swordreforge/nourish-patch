//! `execute()` — the one place a child is actually spawned.

use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

use compositor_introspection_execution_launch_scope::scope::adopt_into_scope;
use compositor_introspection_execution_launch_types::types::{LaunchOutcome, LaunchRequest};

/// Spawn `req` and return its outcome. The PID is always `Child::id()` — we
/// self-spawn, so it is available synchronously and never polled out of systemd.
/// When `scope` is set the live PID is additionally adopted into a transient
/// systemd `.scope` (best-effort); the caller passes `false` when systemd is
/// unavailable (e.g. a sandbox without systemd as PID 1). The reaper owns
/// reaping; we never `wait` on the child here.
pub fn execute(req: &LaunchRequest, scope: bool) -> LaunchOutcome {
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

    // Reset the child's signal mask to empty before exec. We block SIGCHLD
    // process-wide for the reaper's signalfd, and `execve` preserves the mask —
    // so without this, launched apps inherit a blocked SIGCHLD and ones that
    // rely on it (e.g. alacritty detecting its shell exiting) never close.
    // SAFETY: the closure runs in the forked child before exec; sigemptyset /
    // pthread_sigmask are async-signal-safe.
    unsafe {
        cmd.pre_exec(|| {
            let mut set: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut set);
            libc::pthread_sigmask(libc::SIG_SETMASK, &set, std::ptr::null_mut());
            Ok(())
        });
    }

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

    if scope {
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
