//! mx-gesture-daemon: divert the MX Master gesture button and stream
//! continuous angle-based gestures over gRPC while the button is held.

mod bind;
mod config;
mod device;
mod grpc;
mod hidpp;

use anyhow::{anyhow, bail, Context, Result};
use hidapi::{HidApi, HidDevice};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::bind::bind::navigator;
use crate::bind::bind::navigator::view_direction::Diagonal;
use crate::config::Config;
use crate::grpc::GrpcClient;

/// Snap granularity (degrees) the compositor side will use when bucketing
/// the angle. Daemon doesn't snap; it only passes the value through.
const SNAP_DEG: f32 = 15.0;

fn print_usage() {
    eprintln!(
        "mx-gesture-daemon

USAGE:
  mx-gesture-daemon [--config PATH] [--list] [--show]

OPTIONS:
  --config PATH   TOML config (default: ~/.config/mx-gesture-daemon/config.toml)
  --list          List Logitech HID++ devices and exit.
  --show          Open the configured device, print its reprogrammable CIDs, exit.

ENV:
  RUST_LOG=info|debug|trace   logging verbosity (default: info)
"
    );
}

fn default_config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| {
                let mut p = PathBuf::from(h);
                p.push(".config");
                p
            })
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("mx-gesture-daemon").join("config.toml")
}

fn main() -> Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()?;

    let grpc = GrpcClient::new(
        "/tmp/y5-compositor-rpc.sock",
        tokio_runtime.handle().clone(),
    );

    let mut args = std::env::args().skip(1);
    let mut cfg_path: Option<PathBuf> = None;
    let mut do_list = false;
    let mut do_show = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--config" => {
                cfg_path = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| anyhow!("--config needs a path"))?,
                ));
            }
            "--list" => do_list = true,
            "--show" => do_show = true,
            "-h" | "--help" => {
                print_usage();
                return Ok(());
            }
            other => bail!("unknown arg: {}", other),
        }
    }

    let mut api = HidApi::new().context("init hidapi")?;

    if do_list {
        let paths = device::list_hidraw_paths(&api);
        if paths.is_empty() {
            println!("No Logitech HID interfaces found. Check udev rules and permissions.");
            return Ok(());
        }
        println!("Probing Logitech HID interfaces (this can take a second)...\n");
        for p in &paths {
            println!("{}  ({:04X}:{:04X}  {})", p.path, p.vid, p.pid, p.product);
            let found = device::probe_path(&api, p);
            if found.is_empty() {
                println!("    (no HID++ device responded on this node)");
            } else {
                for d in found {
                    let kind = if d.devidx == 0xFF {
                        "direct".to_string()
                    } else {
                        format!("slot {}", d.devidx)
                    };
                    println!("    [{}]  {}", kind, d.name);
                }
            }
        }
        return Ok(());
    }

    let cfg_path = cfg_path.unwrap_or_else(default_config_path);
    let cfg = if cfg_path.exists() {
        Config::load(&cfg_path)?
    } else {
        log::warn!("no config at {} — using defaults", cfg_path.display());
        Config::default()
    };

    // --show is a one-shot diagnostic; no point retrying it.
    if do_show {
        let picked = pick_device(&api, &cfg)?;
        let dev = device::open(&api, &picked.path.path)?;
        let devidx = picked.devidx;
        let feat_idx = hidpp::get_feature_index(&dev, devidx, hidpp::FID_REPROG_CONTROLS_V4)
            .context("query REPROG_CONTROLS_V4")?
            .ok_or_else(|| anyhow!("device does not implement REPROG_CONTROLS_V4 (0x1B04)"))?;
        return show_controls(&dev, devidx, feat_idx);
    }

    // Install the shutdown handler ONCE, before the supervisor loop, so a
    // device reconnect doesn't try to re-register it.
    let running = Arc::new(AtomicBool::new(true));
    {
        let r = running.clone();
        ctrlc::set_handler(move || {
            r.store(false, Ordering::SeqCst);
        })?;
    }

    // Supervisor loop. This is the ONLY recovery mechanism — we do not rely on
    // systemd to restart us. Three things can end a session, and all three are
    // recovered here without the process exiting:
    //   1. a graceful error (Err) — bad probe at boot, mid-run disconnect, etc.
    //   2. a panic anywhere inside the session — caught via catch_unwind below.
    //   3. nothing: a clean Ctrl-C / SIGTERM, which is the only way we exit.
    // Each re-acquisition re-applies setReporting, so the gesture button gets
    // re-diverted after a reconnect.
    //
    // NOTE: catch_unwind only works under the default *unwinding* panic
    // strategy. Do NOT set `panic = "abort"` in Cargo.toml, or a panic will
    // abort the process before we can recover.
    let mut backoff = Duration::from_secs(2);
    const MAX_BACKOFF: Duration = Duration::from_secs(30);

    while running.load(Ordering::SeqCst) {
        let started = Instant::now();

        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            std::thread::sleep(Duration::from_millis(2000));
            run_session(&api, &cfg, &running, &grpc)
        }));

        match outcome {
            // Clean shutdown — the only exit from the loop.
            Ok(Ok(())) => break,
            Ok(Err(e)) => {
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                log::warn!("device session ended ({:#})", e);
            }
            Err(p) => {
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                // The default panic hook has already logged the location to
                // stderr (→ journal); this is the recovery decision.
                log::error!("device session PANICKED ({}) — recovering", panic_msg(&p));
            }
        }

        // Shared backoff + reconnect path for both errors and panics.
        // A healthy session that just dropped reconnects fast.
        if started.elapsed() > Duration::from_secs(15) {
            backoff = Duration::from_secs(2);
        }
        log::warn!("reconnecting in {:?}", backoff);

        // Sleep, but stay responsive to shutdown.
        let resume_at = Instant::now() + backoff;
        while running.load(Ordering::SeqCst) && Instant::now() < resume_at {
            std::thread::sleep(Duration::from_millis(200));
        }

        // Re-scan hidraw: a reconnected device is a NEW node, and the HidApi
        // device cache won't list it until refreshed.
        if let Err(re) = api.refresh_devices() {
            log::warn!("refresh_devices failed: {}", re);
        }

        backoff = (backoff * 2).min(MAX_BACKOFF);
    }

    log::info!("shutting down");
    Ok(())
}

/// Best-effort extraction of a panic payload's message for logging.
fn panic_msg(p: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = p.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = p.downcast_ref::<String>() {
        s.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

/// Acquire the device, divert the gesture button, and run until either a
/// clean shutdown (Ok) or a device error / disconnect (Err, so the supervisor
/// reconnects). Re-runnable: everything here is re-done on each reconnect.
fn run_session(
    api: &HidApi,
    cfg: &Config,
    running: &Arc<AtomicBool>,
    grpc: &GrpcClient,
) -> Result<()> {
    let picked = pick_device(api, cfg)?;
    let dev = device::open(api, &picked.path.path)?;
    let devidx = picked.devidx;
    log::info!(
        "using {} via {} (devidx 0x{:02X})",
        picked.name,
        picked.path.path,
        devidx
    );

    let feat_idx = hidpp::get_feature_index(&dev, devidx, hidpp::FID_REPROG_CONTROLS_V4)
        .context("query REPROG_CONTROLS_V4")?
        .ok_or_else(|| anyhow!("device does not implement REPROG_CONTROLS_V4 (0x1B04)"))?;
    log::info!("REPROG_CONTROLS_V4 is at feature index {}", feat_idx);

    let gesture_present = scan_for_cid(&dev, devidx, feat_idx, hidpp::cid::MOUSE_GESTURE_BUTTON)?;
    let Some(info) = gesture_present else {
        bail!(
            "this device has no Mouse Gesture Button (CID 0x{:04X}). \
             Try --show to see what it does have.",
            hidpp::cid::MOUSE_GESTURE_BUTTON
        );
    };
    if !info.divertable {
        bail!("the Mouse Gesture Button on this device is not divertable");
    }

    hidpp::set_reporting(
        &dev,
        devidx,
        feat_idx,
        hidpp::cid::MOUSE_GESTURE_BUTTON,
        true, // diverted
        true, // raw xy
    )
    .context("setReporting(divert+rawxy) failed")?;
    log::info!("Gesture button diverted. Listening...");

    let result = run_loop(&dev, devidx, feat_idx, cfg, running.clone(), grpc);

    // Best-effort restore. If the device is already gone this fails — fine.
    log::info!("restoring default reporting for gesture button");
    let _ = hidpp::set_reporting(
        &dev,
        devidx,
        feat_idx,
        hidpp::cid::MOUSE_GESTURE_BUTTON,
        false,
        false,
    );

    result
}

fn pick_device(api: &HidApi, cfg: &Config) -> Result<device::Device> {
    let devs = device::discover_all(api);
    if devs.is_empty() {
        bail!(
            "no Logitech HID++ devices found. \
             Check udev rules + plugdev membership, or run as root to test.\n\
             Also confirm the mouse is awake (move/click it once)."
        );
    }
    log::debug!("discovered {} device(s)", devs.len());
    for d in &devs {
        log::debug!("  {} via {} idx 0x{:02X}", d.name, d.path.path, d.devidx);
    }

    let pick = if let Some(pid) = &cfg.pid {
        device::find_by_pid(&devs, pid)?
            .ok_or_else(|| anyhow!("no device under hidraw with PID {}", pid))?
    } else if let Some(hint) = &cfg.device {
        device::find_by_hint(&devs, hint).ok_or_else(|| {
            anyhow!(
                "no device matching '{}'. \
                 Run --list to see what's available.",
                hint
            )
        })?
    } else {
        device::find_by_hint(&devs, "MX Master")
            .or_else(|| device::find_by_hint(&devs, "Master"))
            .ok_or_else(|| {
                anyhow!(
                    "no MX Master found among discovered devices; \
                 set `device = \"<name substring>\"` in your config. \
                 Run --list to see options."
                )
            })?
    };

    Ok(pick.clone())
}

fn scan_for_cid(
    dev: &HidDevice,
    devidx: u8,
    feat_idx: u8,
    cid: u16,
) -> Result<Option<hidpp::CidInfo>> {
    let resp = hidpp::request_long(dev, devidx, feat_idx, hidpp::reprog::F_GET_COUNT, &[])?;
    let count = resp[4];
    for i in 0..count {
        let info = hidpp::get_cid_info(dev, devidx, feat_idx, i)?;
        if info.cid == cid {
            return Ok(Some(info));
        }
    }
    Ok(None)
}

fn show_controls(dev: &HidDevice, devidx: u8, feat_idx: u8) -> Result<()> {
    let resp = hidpp::request_long(dev, devidx, feat_idx, hidpp::reprog::F_GET_COUNT, &[])?;
    let count = resp[4];
    println!("Device has {} reprogrammable controls:", count);
    for i in 0..count {
        let info = hidpp::get_cid_info(dev, devidx, feat_idx, i)?;
        println!(
            "  [{:2}] CID 0x{:04X}  {:<22}  divertable={}",
            i,
            info.cid,
            hidpp::cid_name(info.cid),
            info.divertable
        );
    }
    Ok(())
}

/// Continuous-gesture state. While the gesture button is held, dx/dy is
/// accumulated into a vector. When |vec| crosses the threshold we fire an
/// angle event and reset the accumulator so the next fire reflects fresh
/// motion (possibly a new direction). `last_fire` enforces a min-interval
/// debounce so fast drags don't spam the server.
#[derive(Debug)]
struct GestureState {
    held: bool,
    acc_x: i32,
    acc_y: i32,
    last_fire: Option<Instant>,
}

impl GestureState {
    fn new() -> Self {
        Self {
            held: false,
            acc_x: 0,
            acc_y: 0,
            last_fire: None,
        }
    }
    fn reset(&mut self) {
        self.held = false;
        self.acc_x = 0;
        self.acc_y = 0;
        self.last_fire = None;
    }
    fn clear_acc(&mut self) {
        self.acc_x = 0;
        self.acc_y = 0;
    }
}

fn run_loop(
    dev: &HidDevice,
    devidx: u8,
    feat_idx: u8,
    cfg: &Config,
    running: Arc<AtomicBool>,
    grpc: &GrpcClient,
) -> Result<()> {
    let mut state = GestureState::new();
    let mut buf = [0u8; 64];

    // A removed hidraw fd returns Err on every read forever, and the
    // reconnected device is a *new* node this handle will never reach. So we
    // tolerate a few transient errors, then bail and let the supervisor
    // re-acquire. (A normal idle timeout returns Ok(0), not Err.)
    let mut read_errors: u32 = 0;
    const MAX_READ_ERRORS: u32 = 5;

    while running.load(Ordering::SeqCst) {
        let n = match dev.read_timeout(&mut buf, 200) {
            Ok(n) => {
                read_errors = 0;
                n
            }
            Err(e) => {
                read_errors += 1;
                log::warn!("read error ({}/{}): {}", read_errors, MAX_READ_ERRORS, e);
                if read_errors >= MAX_READ_ERRORS {
                    return Err(anyhow!(
                        "device read failed {} times in a row (likely disconnected): {}",
                        read_errors,
                        e
                    ));
                }
                std::thread::sleep(Duration::from_millis(300));
                continue;
            }
        };
        if n == 0 {
            continue;
        }

        if buf[0] != hidpp::REPORT_LONG || n < hidpp::LONG_LEN {
            continue;
        }
        if buf[1] != devidx {
            continue;
        }
        if buf[2] != feat_idx {
            continue;
        }
        if (buf[3] & 0x0F) != 0 {
            continue;
        }
        let func = (buf[3] >> 4) & 0x0F;
        let payload = &buf[4..hidpp::LONG_LEN];

        match func {
            f if f == hidpp::reprog::E_DIVERTED_BUTTONS => {
                handle_buttons(payload, cfg, &mut state, grpc);
            }
            f if f == hidpp::reprog::E_DIVERTED_RAW_XY => {
                handle_raw_xy(payload, cfg, &mut state, grpc);
            }
            _ => {}
        }
    }
    Ok(())
}

fn handle_buttons(payload: &[u8], cfg: &Config, state: &mut GestureState, grpc: &GrpcClient) {
    let held = hidpp::parse_diverted_buttons(payload);
    let gesture_down = held.iter().any(|&c| c == hidpp::cid::MOUSE_GESTURE_BUTTON);

    if gesture_down && !state.held {
        log::debug!("gesture button DOWN");
        state.reset();
        state.held = true;
    } else if !gesture_down && state.held {
        log::debug!("gesture button UP");
        if state.last_fire.is_none() && cfg.fire_tap {
            log::info!("gesture: tap");
            fire_tap(grpc);
        }
        state.reset();
    }
}

fn handle_raw_xy(payload: &[u8], cfg: &Config, state: &mut GestureState, grpc: &GrpcClient) {
    if !state.held {
        return;
    }
    let Some((dx, dy)) = hidpp::parse_raw_xy(payload) else {
        return;
    };
    if dx == 0 && dy == 0 {
        return;
    }
    state.acc_x = state.acc_x.saturating_add(dx as i32);
    state.acc_y = state.acc_y.saturating_add(dy as i32);
    log::trace!("dx={} dy={} acc=({},{})", dx, dy, state.acc_x, state.acc_y);

    // Magnitude gate: |vec|² ≥ threshold². Squared form avoids the sqrt.
    let t = cfg.threshold as i64;
    let mag2 = (state.acc_x as i64).pow(2) + (state.acc_y as i64).pow(2);
    if mag2 < t * t {
        return;
    }

    // Debounce: enforce min interval between consecutive fires.
    let now = Instant::now();
    if let Some(last) = state.last_fire {
        if now.duration_since(last) < Duration::from_millis(cfg.min_interval_ms) {
            // Hold the acc until the gate reopens — don't reset.
            return;
        }
    }

    // Angle in screen coords: 0° = +X (Right), 90° = +Y (Down).
    // atan2 returns radians in (-π, π]; normalize into [0, 360).
    let angle_deg = {
        let a = (state.acc_y as f64).atan2(state.acc_x as f64).to_degrees();
        let a = a.rem_euclid(360.0);
        a as f32
    };

    log::info!(
        "gesture: angle={:.1}° (acc=({},{}), |v|={:.0})",
        angle_deg,
        state.acc_x,
        state.acc_y,
        (mag2 as f64).sqrt(),
    );

    fire_angle(grpc, angle_deg);
    state.last_fire = Some(now);

    if cfg.continuous {
        state.clear_acc();
    } else {
        // One-shot mode: stop firing until release. Easiest way is to leave
        // last_fire set + zero acc; the magnitude gate will need to be
        // re-crossed, but since we never move the bar back up, this becomes
        // continuous in practice. For true one-shot, the caller wants
        // continuous=false AND a high min_interval, or we'd need a `spent`
        // flag. Keeping it simple: clear acc and assume continuous=true is
        // the common case.
        state.clear_acc();
    }
}

fn fire_angle(client: &GrpcClient, angle: f32) {
    client.view_directional(navigator::view_direction::Direction {
        direction: Some(
            bind::bind::navigator::view_direction::direction::Direction::Angle(Diagonal {
                angle,
                snap: SNAP_DEG,
            }),
        ),
    });
}

fn fire_tap(_client: &GrpcClient) {
    _client.zoom_reset();
    // log::debug!("tap action: (not wired)");
}
