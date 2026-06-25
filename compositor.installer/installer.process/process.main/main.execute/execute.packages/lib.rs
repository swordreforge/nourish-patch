//! Installer step 1: detect the GPU, prompt the dnf package groups, install the
//! selection, and — on NVIDIA — check the bound driver and warn (Nourish never
//! installs the proprietary NVIDIA driver).

use compositor_installer_process_config_parse_base::prompt;
use compositor_installer_process_packages_enumerate_base as pkg;
use compositor_installer_process_packages_enumerate_base::NvidiaDriver;

/// Package name for hardware VA-API video, which only RPM Fusion (free) carries.
const MESA_VA_FREEWORLD: &str = "mesa-va-drivers-freeworld";

/// Detect the GPU vendor, prompt every package group (Enter keeps the default), and run
/// dnf over the selection. STRICT: a real install failure (including a package the
/// enabled repos don't have) returns `Err` so the caller aborts instead of pressing on
/// with a half-installed system. Anything RPM-Fusion-only is never in the base set — it
/// is the explicit, opt-in `mesa-va-drivers-freeworld` step below.
pub fn select_and_install(dry_run: bool) -> Result<(), String> {
    let gpu = pkg::detect_gpu();
    println!("\nDetected GPU vendor: {gpu:?}");
    let mut selected_packages: Vec<String> = Vec::new();
    println!("\n-- Package groups (Enter keeps the [default]) --");
    for group in pkg::groups() {
        let want = prompt::yes_no(
            group.key,
            &format!("{} — {}", group.title, group.description),
            group.default_on,
        );
        if want {
            selected_packages.extend(group.packages.iter().map(|s| s.to_string()));
        }
    }
    if selected_packages.is_empty() {
        println!("No package groups selected — skipping dnf.");
    } else {
        println!("\nInstalling {} packages via dnf...", selected_packages.len());
        pkg::dnf_install(&selected_packages, dry_run)
            .map_err(|e| format!("package install failed: {e}"))?;
    }

    // Explicit, opt-in hardware video acceleration. Vulkan rendering already works from
    // the base `mesa-vulkan-drivers` above; this is ONLY the VA-API video driver, which
    // Fedora can't ship — so it lives in RPM Fusion. Off by default: only when the user
    // says yes do we enable RPM Fusion and install it (otherwise it's never attempted,
    // so the strict install can't trip over it).
    let want_va = prompt::yes_no(
        "mesa-va-drivers (RPM Fusion)",
        "Hardware video acceleration via mesa-va-drivers-freeworld — needs RPM Fusion \
         (free), which this will enable explicitly. Vulkan rendering does NOT need this; \
         it's only for VA-API video decode/encode (e.g. faster capture)",
        false,
    );
    if want_va {
        pkg::enable_rpmfusion_free(dry_run).map_err(|e| format!("RPM Fusion setup failed: {e}"))?;
        pkg::dnf_install(&[MESA_VA_FREEWORLD.to_string()], dry_run)
            .map_err(|e| format!("{MESA_VA_FREEWORLD} install failed: {e}"))?;
    }

    // Nourish ships no NVIDIA driver; if the hardware is here, just report its state.
    if gpu == pkg::Gpu::Nvidia {
        warn_nvidia_driver(pkg::nvidia_driver_status());
    }
    Ok(())
}

/// Print a prominent warning when an NVIDIA GPU is present without the proprietary
/// driver bound. No-op when the proprietary stack is already active.
fn warn_nvidia_driver(status: NvidiaDriver) {
    let reason = match status {
        NvidiaDriver::Proprietary => return,
        NvidiaDriver::Nouveau => "the open-source 'nouveau' driver is bound to it",
        NvidiaDriver::Missing => "no NVIDIA kernel driver is bound (the proprietary stack is missing)",
    };
    // Bold-yellow full-width banner — hard to miss in the scrollback after dnf's
    // output. Rule lines (no right border) so the double-width ⚠ can't break alignment.
    const Y: &str = "\x1b[1;33m";
    const R: &str = "\x1b[0m";
    const RULE: &str = "════════════════════════════════════════════════════════════════════════";
    eprintln!("\n{Y}{RULE}{R}");
    eprintln!("{Y}  ⚠  NVIDIA GPU detected — the proprietary driver is NOT active{R}");
    eprintln!("{Y}{RULE}{R}");
    eprintln!("An NVIDIA GPU is present, but {reason}.");
    eprintln!("Nourish renders on Vulkan and does NOT install the NVIDIA driver for you — on");
    eprintln!("nouveau it will be slow or unstable. Install the proprietary driver yourself,");
    eprintln!("reboot, then re-run this installer:");
    eprintln!();
    eprintln!("  • RPM Fusion akmod (recommended):");
    eprintln!("      sudo dnf install \\");
    eprintln!("        https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm \\");
    eprintln!("        https://mirrors.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-$(rpm -E %fedora).noarch.rpm");
    eprintln!("      sudo dnf install akmod-nvidia xorg-x11-drv-nvidia-cuda");
    eprintln!("      # wait for the kernel module to build (akmods), then reboot");
    eprintln!("  • or NVIDIA's official .run installer from https://www.nvidia.com/Download/index.aspx");
    eprintln!();
    eprintln!("Verify afterwards with:  nvidia-smi   (and 'lspci -k' should show driver: nvidia)");
    eprintln!("{Y}────────────────────────────────────────────────────────────────────────{R}");
}
