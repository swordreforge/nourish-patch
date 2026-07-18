//! On-demand xwayland-satellite integration.
//!
//! Ported from niri's approach: the compositor dynamically allocates an X11
//! display number, creates the X11 sockets, and spawns xwayland-satellite
//! on-demand when an X11 client connects. The display name is propagated
//! to child processes via [`child_display()`].

use std::io;
use std::os::fd::{AsRawFd, BorrowedFd, OwnedFd};
use std::os::unix::net::UnixListener;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::{OnceLock, RwLock};

use compositor_developer_debug_instance_record::{info, warn};
use rustix::fs::{lstat, mkdir, unlink, OFlags};
use rustix::io::Errno;
use rustix::net::SocketAddrUnix;
use rustix::process::getuid;

/// The dynamically assigned DISPLAY for child processes (e.g. ":0").
/// Set once at startup; read by the executor on every launch.
static CHILD_DISPLAY: OnceLock<RwLock<String>> = OnceLock::new();

/// Get the current DISPLAY value for child processes.
/// Returns `None` if xwayland integration is not active.
pub fn child_display() -> Option<String> {
    CHILD_DISPLAY.get()?.read().ok().map(|s| s.clone())
}

/// Set the DISPLAY value (called from setup and on satellite restart).
/// Also propagates to the process environment so the executor's base_env
/// picks it up at install time.
fn set_child_display(display: &str) {
    let _ = CHILD_DISPLAY
        .get_or_init(|| RwLock::new(String::new()))
        .write()
        .map(|mut guard| {
            *guard = display.to_string();
            // Also set in process environment for the executor.
            unsafe { std::env::set_var("DISPLAY", display) };
            info!("xwayland: DISPLAY set to {display} for child processes");
        });
}

struct X11Connection {
    #[allow(dead_code)]
    display_name: String,
    abstract_fd: Option<OwnedFd>,
    unix_fd: OwnedFd,
    _unix_guard: Unlink,
    _lock_guard: Unlink,
}

struct Unlink(String);
impl Drop for Unlink {
    fn drop(&mut self) {
        let _ = unlink(&self.0);
    }
}

const X11_TMP_UNIX_DIR: &str = "/tmp/.X11-unix";

fn ensure_x11_unix_dir() -> io::Result<()> {
    match mkdir(X11_TMP_UNIX_DIR, 0o1777.into()) {
        Ok(()) => Ok(()),
        Err(Errno::EXIST) => {
            ensure_x11_unix_perms()?;
            Ok(())
        }
        Err(err) => Err(io::Error::from(err)),
    }
}

fn ensure_x11_unix_perms() -> io::Result<()> {
    let x11_tmp = lstat(X11_TMP_UNIX_DIR).map_err(io::Error::from)?;
    let tmp = lstat("/tmp").map_err(io::Error::from)?;

    if x11_tmp.st_uid != tmp.st_uid && x11_tmp.st_uid != getuid().as_raw() {
        return Err(io::Error::new(io::ErrorKind::Other, "wrong ownership for X11 directory"));
    }
    if (x11_tmp.st_mode & 0o022) != 0o022 {
        return Err(io::Error::new(io::ErrorKind::Other, "X11 directory is not writable"));
    }
    if (x11_tmp.st_mode & 0o1000) != 0o1000 {
        return Err(io::Error::new(io::ErrorKind::Other, "X11 directory is missing the sticky bit"));
    }

    Ok(())
}

fn pick_x11_display(start: u32) -> io::Result<(u32, OwnedFd, Unlink)> {
    for n in start..start + 50 {
        let lock_path = format!("/tmp/.X{n}-lock");
        let flags = OFlags::WRONLY | OFlags::CLOEXEC | OFlags::CREATE | OFlags::EXCL;
        match rustix::fs::open(&lock_path, flags, 0o444.into()) {
            Ok(lock_fd) => {
                let pid_string = format!("{:>10}\n", rustix::process::getpid().as_raw_nonzero());
                let _ = rustix::io::write(&lock_fd, pid_string.as_bytes());
                return Ok((n, lock_fd, Unlink(lock_path)));
            }
            Err(_) => continue,
        }
    }
    Err(io::Error::new(io::ErrorKind::AddrInUse, "no free X11 display found after 50 attempts"))
}

#[cfg(target_os = "linux")]
fn bind_to_abstract_socket(display: u32) -> io::Result<UnixListener> {
    let name = format!("/tmp/.X11-unix/X{display}");
    let addr = SocketAddrUnix::new_abstract_name(name.as_bytes())?;
    let fd = rustix::net::socket_with(
        rustix::net::AddressFamily::UNIX,
        rustix::net::SocketType::STREAM,
        rustix::net::SocketFlags::CLOEXEC,
        None,
    )?;
    rustix::net::bind(&fd, &addr)?;
    rustix::net::listen(&fd, 1)?;
    Ok(UnixListener::from(fd))
}

fn bind_to_unix_socket(display: u32) -> io::Result<(UnixListener, Unlink)> {
    let name = format!("/tmp/.X11-unix/X{display}");
    let _ = unlink(&name);
    let addr = SocketAddrUnix::new(name.as_bytes())?;
    let fd = rustix::net::socket_with(
        rustix::net::AddressFamily::UNIX,
        rustix::net::SocketType::STREAM,
        rustix::net::SocketFlags::CLOEXEC,
        None,
    )?;
    rustix::net::bind(&fd, &addr)?;
    rustix::net::listen(&fd, 1)?;
    Ok((UnixListener::from(fd), Unlink(name)))
}

fn open_display_sockets(
    display: u32,
) -> io::Result<(Option<UnixListener>, UnixListener, Unlink)> {
    #[cfg(target_os = "linux")]
    let a = Some(bind_to_abstract_socket(display)?);
    #[cfg(not(target_os = "linux"))]
    let a = None;

    let (u, g) = bind_to_unix_socket(display)?;
    Ok((a, u, g))
}

fn setup_connection() -> io::Result<X11Connection> {
    ensure_x11_unix_dir()?;

    let mut n = 0;
    let mut attempt = 0;
    let (display, lock_guard, a, u, unix_guard) = loop {
        let (display, lock_fd, lock_guard) = pick_x11_display(n)?;

        match open_display_sockets(display) {
            Ok((a, u, g)) => break (display, lock_guard, a, u, g),
            Err(err) => {
                if attempt >= 50 {
                    return Err(err);
                }
                n = display + 1;
                attempt += 1;
                continue;
            }
        }
    };

    let display_name = format!(":{display}");
    let abstract_fd = a.map(OwnedFd::from);
    let unix_fd = OwnedFd::from(u);

    Ok(X11Connection {
        display_name,
        abstract_fd,
        unix_fd,
        _unix_guard: unix_guard,
        _lock_guard: lock_guard,
    })
}

/// Spawn xwayland-satellite with `-listenfd` passing the X11 socket fds.
fn spawn_satellite(connection: &X11Connection) {
    let satellite_path = "/usr/bin/xwayland-satellite";

    // Test if satellite supports -listenfd.
    let test = Command::new(satellite_path)
        .args([":0", "--test-listenfd-support"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .env_remove("DISPLAY")
        .env_remove("RUST_BACKTRACE")
        .env_remove("RUST_LIB_BACKTRACE")
        .output();

    match test {
        Ok(out) if !out.status.success() => {
            warn!("xwayland-satellite does not support -listenfd; integration disabled");
            return;
        }
        Err(err) => {
            warn!("failed to test xwayland-satellite -listenfd support: {err}");
            return;
        }
        _ => {}
    }

    let abstract_fd = connection
        .abstract_fd
        .as_ref()
        .map(|fd| fd.try_clone().unwrap());
    let unix_fd = connection.unix_fd.try_clone().unwrap();

    let mut process = Command::new(satellite_path);
    process
        .arg(&connection.display_name)
        .env_remove("DISPLAY")
        .env_remove("RUST_BACKTRACE")
        .env_remove("RUST_LIB_BACKTRACE")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // Pass socket fds via -listenfd.
    let unix_raw = unix_fd.as_raw_fd();
    process.arg("-listenfd").arg(unix_raw.to_string());

    let abstract_raw = abstract_fd.as_ref().map(|fd| fd.as_raw_fd());
    if let Some(raw) = abstract_raw {
        process.arg("-listenfd").arg(raw.to_string());
    }

    // Clear CLOEXEC on the fds before exec so satellite can inherit them.
    unsafe {
        process.pre_exec(move || {
            let unix = BorrowedFd::borrow_raw(unix_raw);
            rustix::io::fcntl_setfd(unix, rustix::io::FdFlags::empty())?;

            if let Some(raw) = abstract_raw {
                let ab = BorrowedFd::borrow_raw(raw);
                rustix::io::fcntl_setfd(ab, rustix::io::FdFlags::empty())?;
            }

            Ok(())
        });
    }

    // Spawn in a background thread to avoid blocking the event loop.
    let display_name = connection.display_name.clone();
    std::thread::Builder::new()
        .name("Xwl-s Spawner".to_owned())
        .spawn(move || {
            // spawn() must happen BEFORE dropping the fds — pre_exec needs
            // them alive to clear CLOEXEC. OwnedFds are dropped automatically
            // when this closure ends.
            match process.spawn() {
                Ok(mut child) => {
                    info!("xwayland-satellite spawned on {display_name} (pid {})", child.id());

                    // Set the DISPLAY for child processes.
                    set_child_display(&display_name);

                    // Wait for satellite to exit, then clear display.
                    let _ = child.wait();
                    warn!("xwayland-satellite on {display_name} exited");
                    // Don't clear DISPLAY — apps that already have it can keep using it.
                    // A future improvement could respawn here.
                }
                Err(err) => {
                    warn!("failed to spawn xwayland-satellite: {err}");
                }
            }
        })
        .map_err(|err| {
            warn!("failed to spawn xwayland-satellite thread: {err}");
        })
        .ok();
}

/// Initialize xwayland-satellite integration. Call from main.rs after the
/// backend is wired and WAYLAND_DISPLAY is set.
pub fn setup() {
    let connection = match setup_connection() {
        Ok(c) => c,
        Err(err) => {
            warn!("xwayland: failed to create X11 sockets: {err}; integration disabled");
            return;
        }
    };

    info!(
        "xwayland: allocated display {} — spawning satellite",
        connection.display_name
    );

    spawn_satellite(&connection);
}
