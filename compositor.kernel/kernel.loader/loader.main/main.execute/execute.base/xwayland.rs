//! On-demand xwayland-satellite integration.
//!
//! The compositor dynamically allocates an X11 display number, creates the X11
//! sockets, and spawns xwayland-satellite as a child process with the socket fds
//! passed via `-listenfd`.  The connection (with its Unlink drop guards) is
//! stored in a static so socket/lock files persist for the compositor's lifetime.

use std::io;
use std::os::fd::{AsRawFd, BorrowedFd, OwnedFd, RawFd};
use std::os::unix::net::UnixListener;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::{OnceLock, RwLock};

use compositor_developer_debug_instance_record::{info, trace, warn};
use rustix::fs::{lstat, mkdir, unlink, OFlags};
use rustix::io::Errno;
use rustix::net::SocketAddrUnix;
use rustix::process::getuid;

static CHILD_DISPLAY: OnceLock<RwLock<String>> = OnceLock::new();
static X11_CONNECTION: OnceLock<X11Connection> = OnceLock::new();

pub fn child_display() -> Option<String> {
    CHILD_DISPLAY.get()?.read().ok().map(|s| s.clone())
}

fn set_child_display(display: &str) {
    let lock = CHILD_DISPLAY.get_or_init(|| RwLock::new(String::new()));
    match lock.write() {
        Ok(mut guard) => {
            *guard = display.to_string();
            unsafe { std::env::set_var("DISPLAY", display) };
            info!("xwayland: DISPLAY={display}");
        }
        Err(poisoned) => {
            warn!("xwayland: CHILD_DISPLAY lock poisoned; recovering");
            let mut guard = poisoned.into_inner();
            *guard = display.to_string();
            unsafe { std::env::set_var("DISPLAY", display) };
            info!("xwayland: DISPLAY={display} (recovered from poison)");
        }
    }
}

struct X11Connection {
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
        trace!("xwayland: unlinked {}", self.0);
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

fn pid_alive(pid: u32) -> bool {
    std::fs::metadata(format!("/proc/{pid}")).is_ok()
}

fn check_stale_lock(lock_path: &str) -> bool {
    let content = match std::fs::read_to_string(lock_path) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let pid: u32 = match content.trim().parse() {
        Ok(p) => p,
        Err(_) => return true,
    };
    if pid == 0 {
        return true;
    }
    !pid_alive(pid)
}

fn cleanup_stale_display(n: u32) {
    let lock_path = format!("/tmp/.X{n}-lock");
    if check_stale_lock(&lock_path) {
        warn!("xwayland: removing stale lock {lock_path}");
        let _ = std::fs::remove_file(&lock_path);
        let socket_path = format!("/tmp/.X11-unix/X{n}");
        if std::fs::metadata(&socket_path).is_ok() {
            warn!("xwayland: removing stale socket {socket_path}");
            let _ = std::fs::remove_file(&socket_path);
        }
    }
}

fn pick_x11_display(start: u32) -> io::Result<(u32, OwnedFd, Unlink)> {
    for n in start..start + 50 {
        cleanup_stale_display(n);
        let lock_path = format!("/tmp/.X{n}-lock");
        let flags = OFlags::WRONLY | OFlags::CLOEXEC | OFlags::CREATE | OFlags::EXCL;
        match rustix::fs::open(&lock_path, flags, 0o444.into()) {
            Ok(lock_fd) => {
                let pid_str = format!("{:>10}\n", rustix::process::getpid().as_raw_nonzero());
                let _ = rustix::io::write(&lock_fd, pid_str.as_bytes());
                return Ok((n, lock_fd, Unlink(lock_path)));
            }
            Err(_) => continue,
        }
    }
    Err(io::Error::new(io::ErrorKind::AddrInUse, "no free X11 display after 50 attempts"))
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
                warn!("xwayland: open sockets failed for display {display}: {err}");
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
    info!("xwayland: allocated display {display_name}");
    Ok(X11Connection { display_name, abstract_fd, unix_fd, _unix_guard: unix_guard, _lock_guard: lock_guard })
}

fn spawn_satellite(connection: &X11Connection) {
    let satellite_path = "/usr/bin/xwayland-satellite";
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
            warn!("xwayland-satellite: no -listenfd support; integration disabled");
            return;
        }
        Err(err) => {
            warn!("xwayland: cannot test satellite -listenfd: {err}");
            return;
        }
        _ => {}
    }

    let abstract_fd = connection.abstract_fd.as_ref().map(|fd| fd.try_clone().unwrap());
    let unix_fd = connection.unix_fd.try_clone().unwrap();

    let mut process = Command::new(satellite_path);
    process
        .arg(&connection.display_name)
        .env_remove("DISPLAY")
        .env_remove("RUST_BACKTRACE")
        .env_remove("RUST_LIB_BACKTRACE")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    let unix_raw = unix_fd.as_raw_fd();
    process.arg("-listenfd").arg(unix_raw.to_string());
    let abstract_raw = abstract_fd.as_ref().map(|fd| fd.as_raw_fd());
    if let Some(raw) = abstract_raw {
        process.arg("-listenfd").arg(raw.to_string());
    }

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

    let display_name = connection.display_name.clone();
    std::thread::Builder::new()
        .name("Xwl-s Spawner".to_owned())
        .spawn(move || {
            let mut child = match process.spawn() {
                Ok(c) => c,
                Err(err) => {
                    warn!("xwayland: spawn satellite failed: {err}");
                    return;
                }
            };
            info!("xwayland: satellite pid={} display={display_name}", child.id());
            set_child_display(&display_name);

            let stderr = child.stderr.take();
            if let Some(stderr) = stderr {
                let display_for_log = display_name.clone();
                std::thread::spawn(move || {
                    use std::io::BufRead;
                    let reader = std::io::BufReader::new(stderr);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            info!("xwayland-satellite[{display_for_log}]: {line}");
                        }
                    }
                });
            }

            let status = child.wait();
            warn!("xwayland: satellite on {display_name} exited: {status:?}");
        })
        .ok();
}

pub fn setup() {
    let connection = match setup_connection() {
        Ok(c) => c,
        Err(err) => {
            warn!("xwayland: socket setup failed: {err}; integration disabled");
            return;
        }
    };
    info!("xwayland: spawning satellite on {}", connection.display_name);
    spawn_satellite(&connection);
    let _ = X11_CONNECTION.set(connection);
}
