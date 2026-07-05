//! NixOS support — declarative, NOT a transactional install.
//!
//! NixOS is non-FHS: a prebuilt, dynamically-linked binary can't even find its ELF
//! interpreter (`/lib64/ld-linux-*.so`), let alone its libraries, so there is nothing to
//! `pacman -S`. The idiomatic fix is `programs.nix-ld` — it provides the interpreter at
//! the standard path and exposes the listed libraries via `NIX_LD_LIBRARY_PATH`. So on
//! NixOS the installer does NOT run a package command; it **prints a `configuration.nix`
//! snippet** (the runtime libs as nixpkgs attributes + nix-ld enablement) and tells the
//! user how to apply it. The package "names" here are therefore nixpkgs attribute names.
//! Pure std.

use compositor_installer_process_packages_enumerate_model::PackageGroup;

/// nixpkgs attribute groups. `_release` is unused (NixOS channels don't rename attrs by
/// soversion). The same group keys as the other tables so the prompt UX is identical; the
/// renderer below routes `runtime`/`devtool` → nix-ld libraries and the rest → systemPackages.
pub fn groups(_release: Option<&str>) -> Vec<PackageGroup> {
    vec![
        PackageGroup {
            key: "runtime",
            title: "y5 runtime libraries (required)",
            description: "Shared libs the prebuilt compositor needs, exposed to it via \
                          nix-ld: Wayland, input/seat/udev, GBM/DRM, Vulkan/EGL, FFmpeg",
            packages: vec![
                "pam", "dbus", "libpulseaudio", "systemd",
                "libinput", "seatd", "libxkbcommon", "pixman",
                "mesa", "libdrm", "libdisplay-info", "ffmpeg",
                "wayland", "vulkan-loader", "libglvnd", "libGL",
            ],
            default_on: true,
        },
        PackageGroup {
            key: "xwayland",
            title: "XWayland / X11 compatibility",
            description: "Run X11 clients under the compositor",
            packages: vec!["xwayland"],
            default_on: true,
        },
        PackageGroup {
            key: "devtool",
            title: "Developer tool window (log viewer)",
            description: "WebKitGTK / GTK runtime libs for the prebuilt dev window",
            packages: vec![
                "webkitgtk_4_1", "gtk3", "libsoup_3", "librsvg",
                "libayatana-appindicator", "glib-networking",
            ],
            default_on: true,
        },
        PackageGroup {
            key: "diagnostics",
            title: "Diagnostics & terminals (optional)",
            description: "vulkan/gl info tools and a terminal",
            packages: vec!["vulkan-tools", "mesa-demos", "foot", "wev"],
            default_on: false,
        },
    ]
}

/// Render the `configuration.nix` snippet from the user's selected groups: `runtime` +
/// `devtool` packages become `programs.nix-ld.libraries`; the rest (`xwayland`,
/// `diagnostics`) become `environment.systemPackages`. Returns the snippet only — the
/// caller prints the surrounding "how to apply" instructions (see execute.packages).
pub fn render_profile(selected: &[PackageGroup]) -> String {
    let pick = |lib: bool| -> String {
        selected
            .iter()
            .filter(|g| is_library_group(g.key) == lib)
            .flat_map(|g| g.packages.iter().copied())
            .map(|p| format!("    {p}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let libs = pick(true);
    let progs = pick(false);
    format!(
        "{{ pkgs, ... }}:\n\
         {{\n\
         \x20 # Let the prebuilt (FHS) y5 binaries find their loader + libraries.\n\
         \x20 programs.nix-ld.enable = true;\n\
         \x20 programs.nix-ld.libraries = with pkgs; [\n\
         {libs}\n\
         \x20 ];\n\
         \x20 environment.systemPackages = with pkgs; [\n\
         {progs}\n\
         \x20 ];\n\
         }}\n"
    )
}

/// Whether a group's packages are shared libraries (→ nix-ld) vs standalone programs
/// (→ systemPackages).
fn is_library_group(key: &str) -> bool {
    matches!(key, "runtime" | "devtool")
}
