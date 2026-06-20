# Installing y5

The y5 compositor ships as a **prebuilt release tarball** for **Fedora 44**. Because
the binaries are already compiled, installing pulls only the runtime shared
libraries — no Rust toolchain, no `-devel` headers.

## One command

```bash
curl -fsSL https://nourish.snowies.com/release/latest/fedora44/package.tar.gz | tar -xz && y5-install/install.sh
```

That downloads the release, unpacks it to `./y5-install/`, and launches the
interactive installer. It uses `sudo` for the system-level steps and is safe to
re-run (every file is overwritten).

If you host the bootstrap script ([`get.sh`](get.sh)) at a stable URL, the command
becomes even shorter:

```bash
curl -fsSL https://nourish.snowies.com/install | bash
```

### Preview without changing anything

```bash
curl -fsSL https://nourish.snowies.com/release/latest/fedora44/package.tar.gz | tar -xz && y5-install/install.sh --dry-run
```

After install, log out and pick a **"Y5…"** session in your display manager.

## What the installer does

1. **Detects your GPU** (`lspci`) and pre-selects the matching driver group.
2. **Installs the runtime packages** (see below) via `dnf`.
3. Prompts the default Y5 Desktop configuration (render node, color depth, VRR, …).
4. Lays down every session preset with its systemd user service, wayland-session
   entry and xdg-desktop-portal config.
5. Optionally installs the developer tool window, the polkit agent, the MX gesture
   daemon and the PAM lock policy.

## Runtime libraries (what actually gets installed)

These are the **exact** runtime dependencies of the prebuilt `y5_compositor`
binary, derived from the binary itself — its ELF `NEEDED` entries (directly linked)
plus the sonames it `dlopen`s at runtime (Wayland, Vulkan, EGL):

| Shared library | Fedora package | How the binary uses it |
| --- | --- | --- |
| `libpam.so.0` | `pam` | linked |
| `libdbus-1.so.3` | `dbus-libs` | linked |
| `libpulse.so.0` | `pulseaudio-libs` | linked |
| `libudev.so.1` | `systemd-libs` | linked |
| `libinput.so.10` | `libinput` | linked |
| `libseat.so.1` | `libseat` | linked |
| `libxkbcommon.so.0` | `libxkbcommon` | linked |
| `libpixman-1.so.0` | `pixman` | linked |
| `libgbm.so.1` | `mesa-libgbm` (→ `libdrm`) | linked |
| `libwayland-{client,server}.so.0` | `libwayland-client`, `libwayland-server` | dlopen |
| `libwayland-egl.so.1` | `mesa-libwayland-egl` | dlopen |
| `libvulkan.so.1` | `vulkan-loader` + driver | dlopen (default renderer) |
| `libEGL.so.1` / GLES | `libglvnd-egl`/`libglvnd-gles` + `mesa-libEGL` | dlopen (GLES fallback) |

The GPU **driver** comes from the vendor group the installer pre-selects:
`mesa-vulkan-drivers`/`mesa-dri-drivers` (AMD/Intel/generic) or the `akmod-nvidia`
stack (NVIDIA). `libdisplay-info` (EDID parsing) and `xorg-x11-server-Xwayland`
(X11 clients) round out the default set.

You do **not** need the build toolchain. The installer offers a `toolchain` group
(off by default) only for the rare case of compiling y5 on the target.

## Building & publishing the artifact

Maintainers build the hostable tarball on a machine with the toolchain:

```bash
compositor.installer/prepare.sh          # -> dist/package.tar.gz + dist/SHA256SUMS
```

Then upload both files to the release path the command above fetches:

```
https://nourish.snowies.com/release/latest/fedora44/package.tar.gz
https://nourish.snowies.com/release/latest/fedora44/SHA256SUMS
```

`get.sh` verifies the tarball against `SHA256SUMS` when it is published.
