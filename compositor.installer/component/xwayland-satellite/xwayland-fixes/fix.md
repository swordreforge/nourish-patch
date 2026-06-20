# xwayland-satellite — y5 fixes for Isaac Sim

This fork of `xwayland-satellite` carries hotfixes for running X11 apps (notably
**NVIDIA Isaac Sim / Omniverse Kit**) under the **y5 compositor** via rootless Xwayland.

Built from upstream `v0.8.1-7-g5d1efbc` (+ y5 patches). Bundled binary:
`dist/xwayland-satellite` (`sha256: a717733a11b009e91af832a97aa46bda5a1eb539dee0fb9ac647e70966c6efbd`).

---

## TL;DR — status

| Symptom | Cause | Resolution |
|---|---|---|
| X apps shrink/grow as integer↔fractional scale changes | DPI-unaware X clients + viewport scale division | `--force-scale 1 --ignore-fractional-scale` (existing) |
| Isaac "move to external window" → **empty/transparent** surface | satellite misclassified the tear-off panel as an `xdg_popup` | **`--popup-fix`** (this fork) — fixed |
| Detached window then shows a **black** surface (one frame, never repaints) | **Isaac Sim 6.0** frame-pacing / window-scheduling bug (not satellite, not y5) | **Workaround:** set a non-"no-pacing" Global Thread Synchronization preset (see below). Upstream-tracked: isaac-sim/IsaacSim #593 |

The compositor (y5) and satellite were proven to deliver everything the client
asks for; the remaining black surface is inside Isaac 6.0. See "Investigation".

---

## The satellite fix: `--popup-fix`

### What was wrong
Isaac's tear-off panels ("Layer", "Render Settings", "Robot Inspector") are
**borderless** X11 top-levels: `_NET_WM_WINDOW_TYPE_NORMAL`, `_MOTIF_WM_HINTS`
with decorations=0 (client-side), no `WM_HINTS` input flag, no `WM_TRANSIENT_FOR`.
satellite's `guess_is_popup()` heuristic (`src/xstate/mod.rs`) read that profile as a
**popup** and gave it an `xdg_popup` role. An `xdg_popup` is a transient child
needing a parent + positioner + grab; mapped as a real window it renders as an
**empty surface** on the host. (Your "these shouldn't be children" instinct was right.)

### What `--popup-fix` does
A window that registers **`WM_DELETE_WINDOW`** is, by definition, an independent
closeable top-level — genuine menus/tooltips/dropdowns/DnD popups never request it.
With `--popup-fix`, such a window is **kept as an `xdg_toplevel`** even if the soft
CSD/skip-taskbar heuristics would otherwise demote it. `override_redirect` and
explicit popup window-types (`_NET_WM_WINDOW_TYPE_MENU/TOOLTIP/…`) are **still**
honored, so real popups are unaffected.

- **Default (flag off): upstream behavior, unchanged.** The fix is a strict no-op
  unless `--popup-fix` is passed.
- Localized to `guess_is_popup()`; verified against the existing popup test suite
  (`popup_heuristics`, `popup_flow_*`, combo/dialog/utility, etc.) — no regressions.

### Usage
```
xwayland-satellite :12 --force-scale 1 --ignore-fractional-scale --popup-fix
```
> `--force-scale` requires the **space-separated** form; `--force-scale=1` is rejected by the parser.

---

## The remaining black surface = Isaac Sim 6.0 (not satellite/y5)

After `--popup-fix` the panel maps as a proper, correctly-sized, input-receiving
window — but Isaac draws **exactly one (black) frame and never repaints it**.

### Workaround (confirmed working)
In Isaac: **Developer → Global Thread Synchronization Preset** → change it away from
**"no-pacing"** to any other preset. This **invalidates the UI and rebuilds Kit's
render/present scheduling**, which finally pulls the detached window into the active
present rotation. It keeps rendering even after you switch the preset back.

- **What "pacing" means:** Kit runs several thread run-loops (UI, render, sim,
  present). The "Global Thread Synchronization Preset" controls how they're paced
  and synchronized per frame. Under **no-pacing**, Kit 6.0 does not add a
  newly-created secondary OS window to the present rotation, so it draws once and
  stalls. Changing the preset re-enumerates windows/swapchains and forces a redraw.
- **Better than toggling by hand:** launch Isaac with a non-"no-pacing" preset from
  the start. The preset is backed by a Carbonite `/app/...` setting (hover the
  control in Preferences to get the exact key), set it via `--/<that.key>=<value>`
  on the Isaac launch command or in your experience `.kit` `[settings]`.
- Tracking upstream: **isaac-sim/IsaacSim #593** ("move to external window" regressed
  in 6.0; broken even on a bare X11 desktop, i.e. independent of Wayland/Xwayland).

### Note on streaming
Headless/WebRTC streaming has **no OS windows** (`--no-window`) and streams the single
composited Kit surface — so it cannot "detach to an external window". It only helps as
an everything-docked fallback.

---

## Investigation (evidence the black surface is not satellite/y5)

With diagnostic logging (since removed) on the torn-off surface vs. the main window:

- **Role:** satellite logs `creating toplevel` for the panel (not popup) with `--popup-fix`.
- **Geometry:** at `--force-scale 1` the panel is **1:1** — X size = buffer size =
  viewport dst = `423x374`. The *main* window has *more* manipulation (viewport dst
  shaved 25px for its titlebar) yet renders fine → not a geometry/"size lie" bug.
- **Buffers:** the panel receives exactly **1 buffer attach + commit**; the main
  window receives thousands. No stimulus (resize, obscure/expose, click, move)
  produced a 2nd buffer.
- **Frame callbacks:** panel `requested=1, fired=1` — the host (y5) **fired** the
  frame callback and satellite forwarded the `done`. Isaac got its green light and
  still didn't draw frame 2. (Rules out frame-callback starvation.)
- **Control:** Blender's second GPU window renders fine through the same satellite+y5
  path. Only Isaac 6.0's detached window misbehaves — and the Kit pacing toggle (an
  internal render-rotation rebuild) is what fixes it.

Conclusion: satellite and y5 deliver buffers, callbacks, geometry, and X events
faithfully. The missing second frame originates in Isaac/Kit 6.0.

---

## Build

Toolchain: stable Rust (edition 2024). Uses the `mold` linker via repo `.cargo/config.toml`
— do **not** set `RUSTFLAGS` (it drops the linker config).

```
cd xwayland-fixes
cargo build --release          # -> target/release/xwayland-satellite
cargo test --release --lib     # 55 unit tests
```

A prebuilt binary is bundled at `dist/xwayland-satellite`.

## Deploy

```
sudo install -m755 dist/xwayland-satellite /usr/bin/xwayland-satellite
# user systemd unit (see xwayland.service in this folder):
install -m644 xwayland.service ~/.config/systemd/user/xwayland-satellite.service
systemctl --user daemon-reload
systemctl --user restart xwayland-satellite
```

`xwayland.service` (in this folder) launches:
```
/usr/bin/xwayland-satellite :12 --force-scale 1 --ignore-fractional-scale --popup-fix
```
Be sure X11 clients (Isaac Sim) get `DISPLAY=:12`.

---

## Files in this folder

- `dist/xwayland-satellite` — bundled release binary (`--popup-fix` capable).
- `xwayland.service` — updated systemd user unit (adds `--popup-fix`, `--force-scale 1`).
- `awk.sh` — frame-callback / buffer diagnostic for satellite debug logs (investigation aid).
- `fix.md` — this document.
