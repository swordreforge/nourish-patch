//! Compositor-owned IME launch with pgid-based authorization.
use std::io::BufRead;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use smithay::reexports::wayland_server::{Client, DisplayHandle};
use compositor_developer_environment_preference_base::base::Ime;

static IME_PGID: AtomicI32 = AtomicI32::new(0);
static IME_START: AtomicU64 = AtomicU64::new(0);

pub fn launch(configured: Option<Ime>) {
    let Some(ime) = configured.filter(|i| !i.exec.trim().is_empty()) else {
        info!("ime: none configured — not launching");
        return;
    };
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Ok(d) = std::env::var("DISPLAY") {
            if let Some(n) = d.strip_prefix(':') {
                if UnixStream::connect(format!("/tmp/.X11-unix/X{n}")).is_ok() { break; }
            }
        }
        if Instant::now() > deadline { warn!("x11 not ready, spawning IME anyway"); break; }
        std::thread::sleep(Duration::from_millis(500));
    }
    let spawned = Command::new(&ime.exec).args(&ime.args).process_group(0)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn();
    let mut child = match spawned {
        Ok(c) => c,
        Err(e) => return error!("ime: failed to launch '{}': {e}", ime.exec),
    };
    let pid = child.id() as i32;
    IME_START.store(stat_field(pid, 19).unwrap_or(0), Ordering::SeqCst);
    IME_PGID.store(pid, Ordering::SeqCst);
    info!("ime: launched '{}' (pid/pgid {pid})", ime.exec);
    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stderr).lines().map_while(Result::ok) {
                error!("ime: {line}");
            }
        });
    }
}

pub fn is_authorized(client: &Client, dh: &DisplayHandle) -> bool {
    let auth = IME_PGID.load(Ordering::SeqCst);
    if auth == 0 { return false; }
    let now = stat_field(auth, 19);
    let want = IME_START.load(Ordering::SeqCst);
    if now.is_none() || (want != 0 && now != Some(want)) { return false; }
    let Ok(creds) = client.get_credentials(dh) else { return false; };
    creds.pid == auth || stat_field(creds.pid, 2) == Some(auth as u64)
}

fn stat_field(pid: i32, index: usize) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after = &stat[stat.rfind(')')? + 1..];
    after.split_whitespace().nth(index)?.parse().ok()
}
