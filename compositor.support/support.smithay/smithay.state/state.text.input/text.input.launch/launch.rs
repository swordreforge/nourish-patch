//! The compositor-owned input method. y5 launches exactly one IME process and grants the
//! input-method / virtual-keyboard globals ONLY to that process group. Identity is the spawned
//! pid — never a guessed `/proc/<pid>/exe` (which fails across mount/pid namespaces) nor a
//! spoofable `comm`. Configured by `Preference::ime`.

use std::io::BufRead;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};

use smithay::reexports::wayland_server::{Client, DisplayHandle};

use compositor_developer_environment_preference_base::base::Ime;

/// Process-group id of the launched input method; `0` before launch / if none was started. One
/// compositor-wide value — a single trusted IME per session.
static IME_PGID: AtomicI32 = AtomicI32::new(0);

/// Launch the configured input method as a direct child in its OWN process group, record its pid
/// as the sole authorized IME, and stream its stderr to the log as errors. Call once at startup
/// AFTER `WAYLAND_DISPLAY` is exported (the child inherits it). No IME is configured (or an empty
/// `exec`) launches nothing — there is no built-in default.
pub fn launch(configured: Option<Ime>) {
    let Some(ime) = configured else {
        info!("ime: none configured (preferences.json `ime`) — not launching");
        return;
    };
    if ime.exec.trim().is_empty() {
        info!("ime: configured exec is empty — not launching");
        return;
    }

    // `process_group(0)` puts the child in a new group whose pgid == its pid, so authorizing that
    // one pid covers the IME and any helper it forks. Do NOT let the IME daemonize (`-d`) or the
    // process that connects becomes a reparented grandchild we can't identify.
    let spawned = Command::new(&ime.exec)
        .args(&ime.args)
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match spawned {
        Ok(c) => c,
        Err(e) => {
            error!("ime: failed to launch '{}': {e}", ime.exec);
            return;
        }
    };

    let pid = child.id() as i32;
    IME_PGID.store(pid, Ordering::SeqCst);
    info!("ime: launched '{}' (pid/pgid {pid})", ime.exec);

    // Propagate the IME's stderr to the console as errors. The child is otherwise detached — the
    // std `Child` never kills on drop, and the global SIGCHLD reaper collects it.
    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stderr).lines().map_while(Result::ok) {
                error!("ime: {line}");
            }
        });
    }
}

/// Whether `client` is the launched input method — the ONLY client permitted to bind the
/// input-method / virtual-keyboard globals. True iff its process group equals the spawned pid.
pub fn is_authorized(client: &Client, dh: &DisplayHandle) -> bool {
    let auth = IME_PGID.load(Ordering::SeqCst);
    if auth == 0 {
        return false; // nothing launched → deny everyone
    }
    let Ok(creds) = client.get_credentials(dh) else {
        return false;
    };
    creds.pid == auth || pgid_of(creds.pid) == Some(auth)
}

/// Process-group id of `pid` from `/proc/<pid>/stat` field 5 (`pgrp`). World-readable, so it works
/// where `/proc/<pid>/exe` fails (no ptrace permission needed). `comm` (field 2) can contain
/// spaces and `)`, so read the fixed fields AFTER the last `)`: `state ppid pgrp …`.
fn pgid_of(pid: i32) -> Option<i32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after = &stat[stat.rfind(')')? + 1..];
    after.split_whitespace().nth(2)?.parse::<i32>().ok()
}
