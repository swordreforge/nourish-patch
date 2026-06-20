# mx-gesture-daemon

A tiny single-binary Rust daemon that diverts the **Mouse Gesture Button** on
Logitech MX Master / MX Master 2S / 3 / 3S / 4 via HID++ 2.0, and runs shell
commands when you press-and-drag.

No GUI. No Python. No daemon stack. Just one binary, one TOML file, ~1000 LOC.

## What it does

1. Enumerates `/dev/hidraw*` nodes that belong to Logitech (`046D:*`).
2. For each, probes both **direct addressing** (`0xFF`, used for USB cable
   and Bluetooth) and **receiver slots** (`1..6`, used for Unifying / Bolt /
   Lightspeed receivers). Whichever index responds is a real device.
3. Reads the device name via feature `0x0005` to find your MX Master.
4. Resolves feature `0x1B04` (`REPROG_CONTROLS_V4`) on the selected device.
5. Confirms the **Mouse Gesture Button** (CID `0xC3`) is divertable.
6. Calls `setReporting` to mark it as *diverted* with *raw XY* on, so the
   firmware stops doing its built-in gesture handling and streams dx/dy
   while the button is held.
7. Watches the notification stream, integrates dx/dy until a threshold is
   crossed, picks a 4-way direction, and runs the mapped shell command.
8. On exit (Ctrl-C / SIGTERM), restores the gesture button to its default.

## Build

```bash
cargo build --release
install -Dm755 target/release/mx-gesture-daemon ~/.local/bin/mx-gesture-daemon
```

## Setup

```bash
# udev rule so you don't need root
sudo install -m644 42-logitech-hidpp.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
sudo usermod -aG plugdev "$USER"   # then log out + in

# config
mkdir -p ~/.config/mx-gesture-daemon
cp config.example.toml ~/.config/mx-gesture-daemon/config.toml
$EDITOR ~/.config/mx-gesture-daemon/config.toml
```

## Usage

```bash
mx-gesture-daemon --list           # probe and list real devices behind each hidraw
mx-gesture-daemon --show           # show the device's CIDs and their flags
mx-gesture-daemon                  # run (foreground)

RUST_LOG=debug mx-gesture-daemon   # verbose
```

`--list` prints something like:

```
/dev/hidraw2  (046D:405B  USB Receiver)
    [slot 1]  MX Master 3S
    [slot 2]  MX Keys
/dev/hidraw1  (046D:4082  USB Receiver)
    (no HID++ device responded on this node)
```

If your mouse shows as offline, wake it (move or click it) and re-run.

In your config, set `device = "MX Master 3S"` (or whatever name appeared
in `--list`) to pin selection. The daemon defaults to the first device
whose name contains "MX Master".

## Important caveats

- **Multiple receivers** are supported — discovery probes every hidraw node
  and finds your mouse wherever it is. The first scan may take ~1 second
  while empty slots are probed.
- **Do not run Solaar at the same time** — both tools will fight over the
  divert flag.
- **Diversion may reset on reconnect.** If the mouse sleeps deeply and
  wakes, the daemon may need a restart (`systemctl --user restart ...`).
- **Wayland**: `xdotool` won't work. Use `wtype`, `ydotool`, `dbus-send`, or a
  compositor-specific tool (e.g. `hyprctl`, `swaymsg`, `gdbus`).
- **Direction polarity** (which sign means "up") depends on the device.
  If up/down are inverted, swap the up/down lines in your config.

## Files

```
src/hidpp.rs    HID++ 2.0 framing, request/response, error decoding,
                feature lookup, setReporting, notification parsing
src/device.rs   hidraw enumeration, ping, receiver-slot probing,
                name resolution via feature 0x0005
src/config.rs   TOML parsing
src/main.rs     CLI + main loop
```

## License

MIT.

## Status

This is a clean-room implementation derived from reading Solaar's
`logitech_receiver/hidpp20.py`, the Logitech `cpg-docs` HID++ 2.0
documentation, and the Linux kernel's `hid-logitech-hidpp.c`.

It's been developed iteratively against real device output — bugs are
likely on first run. If something doesn't work, `RUST_LOG=trace` shows
every TX/RX byte and is your best debugging tool.
