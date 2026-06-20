mod server;
pub mod xstate;

use crate::server::{NoConnection, PendingSurfaceState, ServerState};
use crate::xstate::{RealConnection, XState};
use log::{error, info};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use server::selection::{Clipboard, Primary};
use smithay_client_toolkit::data_device_manager::WritePipe;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};
use std::os::unix::{net::UnixStream, process::ExitStatusExt};
use std::process::{Command, ExitStatus, Stdio};
use wayland_server::{Display, ListeningSocket};
use xcb::x;

/// Per-flag pipeline gating for HiDPI/scaling diagnostics.
/// All fields default to `false`/`None`, preserving current behavior when unset.
#[derive(Debug, Clone, Default)]
pub struct ScaleConfig {
    /// --force-scale <f64>: override the derived effective scale S with a constant.
    pub force_scale: Option<f64>,
    /// --ignore-fractional-scale: skip binding wp_fractional_scale_manager; use integer wl_output scale only.
    pub ignore_fractional_scale: bool,
    /// --identity-viewport: set wp_viewport destination to the native X pixel size (no scale division).
    pub identity_viewport: bool,
    /// --no-xsettings-dpi: do not write Xft.dpi/Gdk DPI via XSETTINGS; cede fully to external xsettingsd.
    pub no_xsettings_dpi: bool,
    /// --x-resolution <logical|physical>: dimensions advertised to Xwayland via xdg_output.logical_size.
    pub x_resolution: XResolution,
    /// --log-scale: emit scale/viewport info at INFO level (instead of DEBUG) on each output change and window configure.
    pub log_scale: bool,
    /// --popup-fix: keep a borderless/CSD X11 window that registers WM_DELETE_WINDOW
    /// as an xdg_toplevel instead of letting the popup heuristic demote it to an
    /// xdg_popup (which renders as an empty surface on the host). Needed for apps
    /// like Isaac Sim whose tear-off tool panels are NORMAL, no-decoration windows.
    pub popup_fix: bool,
}

/// Which screen dimensions are advertised to Xwayland via xdg_output.logical_size.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum XResolution {
    /// Advertise the physical pixel dimensions from wl_output.mode (current behavior).
    #[default]
    Physical,
    /// Advertise the logical pixel dimensions from xdg_output.logical_size.
    Logical,
}

pub trait XConnection: Sized + 'static {
    type X11Selection: X11Selection;

    fn set_window_dims(&mut self, window: x::Window, dims: PendingSurfaceState) -> bool;
    fn set_fullscreen(&mut self, window: x::Window, fullscreen: bool);
    fn focus_window(&mut self, window: x::Window, output_name: Option<String>);
    fn close_window(&mut self, window: x::Window);
    fn unmap_window(&mut self, window: x::Window);
    fn raise_to_top(&mut self, window: x::Window);
}

pub trait X11Selection {
    fn mime_types(&self) -> Vec<&str>;
    fn write_to(&self, mime: &str, pipe: WritePipe);
}

type EarlyServerState = ServerState<NoConnection<<RealConnection as XConnection>::X11Selection>>;
type RealServerState = ServerState<RealConnection>;

pub trait RunData {
    fn display(&self) -> Option<&str>;
    fn listenfds(&mut self) -> Vec<OwnedFd>;
    fn flags(&self) -> &[String] {
        &[]
    }
    fn server(&self) -> Option<UnixStream> {
        None
    }
    fn created_server(&self) {}
    fn connected_server(&self) {}
    fn quit_rx(&self) -> Option<UnixStream> {
        None
    }
    fn xwayland_ready(&self, _display: String, _pid: u32) {}
    fn max_req_len_bytes(&self) -> Option<usize> {
        None
    }
    fn scale_config(&self) -> ScaleConfig {
        ScaleConfig::default()
    }
}

pub const fn timespec_from_millis(millis: u64) -> Timespec {
    let d = std::time::Duration::from_millis(millis);
    Timespec {
        tv_sec: d.as_secs() as i64,
        tv_nsec: d.subsec_nanos() as i64,
    }
}

pub fn version() -> &'static str {
    let mut version = env!("VERGEN_GIT_DESCRIBE");
    if version == "VERGEN_IDEMPOTENT_OUTPUT" {
        version = env!("CARGO_PKG_VERSION");
    }
    version
}

pub fn main(mut data: impl RunData) -> Option<()> {
    info!("Starting xwayland-satellite version {}", version());

    let socket = ListeningSocket::bind_auto("xwls", 1..=128).unwrap();
    let mut display = Display::new().unwrap();
    let dh = display.handle();
    data.created_server();

    let (xsock_wl, xsock_xwl) = UnixStream::pair().unwrap();
    // Prevent creation of new Xwayland command from closing fd
    rustix::io::fcntl_setfd(&xsock_xwl, rustix::io::FdFlags::empty()).unwrap();

    let (ready_tx, ready_rx) = UnixStream::pair().unwrap();
    rustix::io::fcntl_setfd(&ready_tx, rustix::io::FdFlags::empty()).unwrap();
    let mut xwayland = Command::new("Xwayland");
    if let Some(display) = data.display() {
        xwayland.arg(display);
    }

    let fds = data.listenfds();
    for fd in &fds {
        xwayland.args(["-listenfd", &fd.as_raw_fd().to_string()]);
    }

    let mut xwayland = xwayland
        .args([
            "-rootless",
            "-force-xrandr-emulation",
            "-wm",
            &xsock_xwl.as_raw_fd().to_string(),
            "-displayfd",
            &ready_tx.as_raw_fd().to_string(),
        ])
        .args(data.flags())
        .env("WAYLAND_DISPLAY", socket.socket_name().unwrap())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // Now that Xwayland spawned and got the listenfds, we can close them here.
    drop(fds);

    let xwl_pid = xwayland.id();

    let (mut finish_tx, mut finish_rx) = UnixStream::pair().unwrap();
    let stderr = xwayland.stderr.take().unwrap();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            let line = line.unwrap();
            info!(target: "xwayland_process", "{line}");
        }
        let status = xwayland.wait().unwrap().into_raw();
        // On a successful integration test, the rx will be dropped, so keep logs/GDB clean
        let _ = finish_tx.write_all(&status.to_ne_bytes());
    });

    let mut ready_fds = [
        PollFd::new(&socket, PollFlags::IN),
        PollFd::new(&finish_rx, PollFlags::IN),
    ];

    fn xwayland_exit_code(rx: &mut UnixStream) -> ExitStatus {
        let mut data = [0; std::mem::size_of::<i32>()];
        rx.read_exact(&mut data).unwrap();
        ExitStatus::from_raw(i32::from_ne_bytes(data))
    }

    let connection = match poll(&mut ready_fds, None) {
        Ok(_) => {
            if !ready_fds[1].revents().is_empty() {
                let status = xwayland_exit_code(&mut finish_rx);
                error!("Xwayland exited early with {status}");
                return None;
            }

            data.connected_server();
            socket.accept().unwrap().unwrap()
        }
        Err(e) => {
            panic!("first poll failed: {e:?}")
        }
    };

    let scale_config = data.scale_config();
    let mut server_state = EarlyServerState::new(dh, data.server(), connection, scale_config.clone());
    server_state.run();

    // Remove the lifetimes on our fds to avoid borrowing issues, since we know they will exist for
    // the rest of our program anyway
    let server_fd = unsafe { BorrowedFd::borrow_raw(server_state.clientside_fd().as_raw_fd()) };
    let display_fd = unsafe { BorrowedFd::borrow_raw(display.backend().poll_fd().as_raw_fd()) };

    // `finish_rx` only writes the status code of `Xwayland` exiting, so it is reasonable to use as
    // the UnixStream of choice when not running the integration tests.
    let mut quit_rx = data.quit_rx().unwrap_or(finish_rx);

    let mut fds = [
        PollFd::from_borrowed_fd(server_fd, PollFlags::IN),
        PollFd::new(&xsock_wl, PollFlags::IN),
        PollFd::from_borrowed_fd(display_fd, PollFlags::IN),
        PollFd::new(&quit_rx, PollFlags::IN),
        PollFd::new(&ready_rx, PollFlags::IN),
    ];

    loop {
        match poll(&mut fds, None) {
            Ok(_) => {
                if !fds[3].revents().is_empty() {
                    let status = xwayland_exit_code(&mut quit_rx);
                    if status != ExitStatus::default() {
                        error!("Xwayland exited early with {status}");
                    }
                    return None;
                }
                if !fds[4].revents().is_empty() {
                    break;
                }
            }
            Err(other) => panic!("Poll failed: {other:?}"),
        }

        display.dispatch_clients(&mut *server_state).unwrap();
        server_state.run();
        display.flush_clients().unwrap();
    }

    let mut xstate = XState::new(xsock_wl.as_fd());
    if let Some(bytes) = data.max_req_len_bytes() {
        xstate.set_max_req_bytes(bytes);
    }

    let mut reader = BufReader::new(&ready_rx);
    {
        let mut display = String::new();
        reader.read_line(&mut display).unwrap();
        display.pop();
        display.insert(0, ':');
        info!("Connected to Xwayland on {display}");
        data.xwayland_ready(display, xwl_pid);
    }
    if scale_config.log_scale {
        info!("ScaleConfig: {:?}", scale_config);
    }
    let mut server_state = xstate.server_state_setup(server_state);

    #[cfg(feature = "systemd")]
    {
        match sd_notify::notify(true, &[sd_notify::NotifyState::Ready]) {
            Ok(()) => info!("Successfully notified systemd of ready state."),
            Err(e) => log::warn!("Systemd notify failed: {e:?}"),
        }
    }

    #[cfg(not(feature = "systemd"))]
    info!("Systemd support disabled.");

    loop {
        xstate.handle_events(&mut server_state);

        display.dispatch_clients(&mut *server_state).unwrap();
        server_state.run();
        display.flush_clients().unwrap();

        if let Some(sel) = server_state.new_selection::<Clipboard>() {
            xstate.set_clipboard(sel);
        }

        if let Some(sel) = server_state.new_selection::<Primary>() {
            xstate.set_primary_selection(sel);
        }

        if let Some(scale) = server_state.new_global_scale() {
            // --no-xsettings-dpi: skip writing Xsettings DPI entirely
            if !scale_config.no_xsettings_dpi {
                xstate.update_global_scale(scale);
            }
        }

        match poll(&mut fds, None) {
            Ok(_) => {
                if !fds[3].revents().is_empty() {
                    let status = xwayland_exit_code(&mut quit_rx);
                    if status != ExitStatus::default() {
                        error!("Xwayland exited early with {status}");
                    }
                    return None;
                }
            }
            Err(other) => panic!("Poll failed: {other:?}"),
        }
    }
}
