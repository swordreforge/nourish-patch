//! dnf package groups, GPU-vendor detection, and the `dnf install` runner.
//!
//! The `runtime` group is the authoritative *runtime* dependency set for the
//! prebuilt `y5_compositor` binary. It is derived from the binary itself, not from
//! guesswork: the directly linked sonames come from its ELF `NEEDED` entries
//! (`readelf -d y5.compositor`), and the `dlopen`-loaded sonames (Wayland, Vulkan,
//! EGL) come from the embedded soname strings. Because the artifact ships a
//! compiled binary, an end user needs only these shared libraries plus a GPU
//! driver — NOT the Rust toolchain or any `-devel` headers. The `toolchain` group
//! (off by default) carries the build-from-source dependencies for the rare case
//! of compiling on the target.
//!
//! soname -> Fedora runtime package (the mapping encoded below):
//!   libpam.so.0         -> pam                 libdbus-1.so.3   -> dbus-libs
//!   libpulse.so.0       -> pulseaudio-libs     libudev.so.1     -> systemd-libs
//!   libgbm.so.1         -> mesa-libgbm         libseat.so.1     -> libseat
//!   libinput.so.10      -> libinput            libxkbcommon.so.0-> libxkbcommon
//!   libpixman-1.so.0    -> pixman
//!   libwayland-{client,server,egl}.so* -> libwayland-{client,server} + mesa-libwayland-egl
//!   libvulkan.so.1      -> vulkan-loader (+ mesa-vulkan-drivers / nvidia ICD)
//!   libEGL.so.1         -> libglvnd-egl (+ mesa-libEGL / nvidia)
//!
//! Pure std.

use compositor_installer_process_packages_enumerate_model::{Gpu, PackageGroup};

/// All selectable groups. `gpu` pre-selects the matching driver group.
///
/// The default selection installs only what the **prebuilt** binary needs at
/// runtime: the `runtime` libs, the GPU driver for the detected vendor, XWayland
/// and the dev tool's runtime libs. Nothing here pulls the Rust toolchain or
/// `-devel` headers — that is the opt-in `toolchain` group, for building from
/// source on the target.
pub fn groups(gpu: Gpu) -> Vec<PackageGroup> {
    vec![
        PackageGroup {
            key: "runtime",
            title: "y5 runtime libraries (required)",
            description: "Exact shared libs the prebuilt compositor links/dlopens: \
                          Wayland, input/seat/udev, GBM/DRM, pixman, Vulkan/EGL loader \
                          + generic Mesa driver, PAM, dbus, PulseAudio",
            packages: vec![
                // Directly linked (ELF NEEDED).
                "pam", "dbus-libs", "pulseaudio-libs", "systemd-libs",
                "libinput", "libseat", "libxkbcommon", "pixman",
                "mesa-libgbm", "libdrm", "libdisplay-info",
                // dlopen'd Wayland libs.
                "libwayland-client", "libwayland-server", "mesa-libwayland-egl",
                // dlopen'd render stack: loaders + dispatch + generic Mesa driver
                // (the vendor-specific driver comes from the matching group below).
                "vulkan-loader", "mesa-vulkan-drivers",
                "libglvnd-egl", "libglvnd-gles", "libglvnd-opengl",
                "mesa-libEGL", "mesa-libGL", "mesa-dri-drivers",
            ],
            default_on: true,
        },
        PackageGroup {
            key: "intel",
            title: "Intel VA-API video acceleration",
            description: "Intel media drivers for hardware video (Intel GPUs)",
            packages: vec!["intel-media-driver", "libva-intel-driver", "mesa-va-drivers"],
            default_on: matches!(gpu, Gpu::Intel),
        },
        PackageGroup {
            key: "amd",
            title: "AMD VA-API video acceleration",
            description: "Mesa VA-API drivers for hardware video (AMD GPUs)",
            packages: vec!["mesa-va-drivers", "libva-utils"],
            default_on: matches!(gpu, Gpu::Amd),
        },
        PackageGroup {
            key: "nvidia",
            title: "NVIDIA proprietary driver stack",
            description: "akmod-nvidia, CUDA, 32-bit libs, NVIDIA VA-API (NVIDIA GPUs)",
            packages: vec![
                "akmod-nvidia", "xorg-x11-drv-nvidia-cuda",
                "xorg-x11-drv-nvidia-libs.i686", "libva-nvidia-driver",
            ],
            default_on: matches!(gpu, Gpu::Nvidia),
        },
        PackageGroup {
            key: "xwayland",
            title: "XWayland / X11 compatibility",
            description: "Run X11 clients under the compositor (runtime only)",
            packages: vec!["xorg-x11-server-Xwayland"],
            default_on: true,
        },
        PackageGroup {
            key: "devtool",
            title: "Developer tool window (log viewer)",
            description: "WebKitGTK / GTK runtime libs for the prebuilt dev window",
            packages: vec![
                "webkit2gtk4.1", "libsoup3", "gtk3",
                "librsvg2", "libappindicator-gtk3",
            ],
            default_on: true,
        },
        PackageGroup {
            key: "diagnostics",
            title: "Diagnostics & terminals (optional)",
            description: "vulkan/egl/glx info tools, glmark2, a couple of terminals",
            packages: vec![
                "vulkan-tools", "egl-utils", "glx-utils", "mesa-demos", "glmark2",
                "foot", "alacritty", "wev",
            ],
            default_on: false,
        },
        PackageGroup {
            key: "toolchain",
            title: "Build-from-source toolchain (NOT needed for the prebuilt install)",
            description: "Rust/cargo, clang, protobuf, and every -devel header — only if \
                          you intend to compile y5 on this machine",
            packages: vec![
                "cargo", "rust", "git", "clang-devel", "pkgconf-pkg-config", "mold",
                "pam-devel", "libdisplay-info-devel", "libinput-devel", "libseat-devel",
                "libxkbcommon-devel", "pixman-devel", "systemd-devel",
                "wayland-devel", "wayland-protocols-devel", "mesa-libgbm-devel",
                "vulkan-loader-devel", "mesa-libEGL-devel", "mesa-libGL-devel",
                "libglvnd-devel", "libX11-devel", "libxcb-devel", "xcb-util-cursor-devel",
                "protobuf", "protobuf-devel", "protobuf-compiler",
                "dbus-devel", "pulseaudio-libs-devel", "openssl-devel",
                "webkit2gtk4.1-devel", "libsoup3-devel", "gtk3-devel",
                "librsvg2-devel", "libappindicator-gtk3-devel", "patchelf",
            ],
            default_on: false,
        },
    ]
}
