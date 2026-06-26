#!/usr/bin/env bash
# Build + run y5_compositor directly on the host — NO container.
#
# The counterpart to run.sh (which runs inside the podman dev container). This
# builds via build.sh and execs the binary on the bare-metal host, so it nests
# into your real Wayland session (winit) or drives DRM/KMS on a TTY (udev).
#
# Usage: ./run-host.sh [winit|udev] [debug|release] [--it] [--env=FILE]
#   winit | udev      backend (default: winit). udev = DRM/KMS, run from a TTY.
#   debug | release   cargo profile (default: debug).
#   --it, -i          interactively prompt for every supported env var, showing
#                     each one's description + current default (Enter = keep it).
#   --env=FILE        source FILE first as the env base before prompting/running,
#                     e.g. --env=../environment.container/container.env for the NVIDIA var set.
#
# Without --it it runs with the defaults below (inheriting your shell env). The
# renderer is chosen at runtime via COMPOSITOR_RENDERER (no rebuild needed):
# default is `vulkan`; set `gles` for the GLES renderer.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

BACKEND=winit
PROFILE=debug
INTERACTIVE=0
ENV_FILE=""
for arg in "$@"; do
    case "$arg" in
        winit | udev | native) BACKEND="$arg" ;;
        debug | release) PROFILE="$arg" ;;
        --it | -i) INTERACTIVE=1 ;;
        --env=*) ENV_FILE="${arg#--env=}" ;;
        -h | --help) sed -n '2,20p' "${BASH_SOURCE[0]}"; exit 0 ;;
        *) echo "run-host.sh: unknown arg '$arg' (see --help)" >&2; exit 1 ;;
    esac
done

# Optional base env file (e.g. the NVIDIA var set). Exported into the environment
# so the defaults below pick the file's values up.
if [ -n "$ENV_FILE" ]; then
    [ -f "$ENV_FILE" ] || ENV_FILE="$HERE/$ENV_FILE"
    [ -f "$ENV_FILE" ] || { echo "run-host.sh: env file not found: $ENV_FILE" >&2; exit 1; }
    echo ">> sourcing env base: $ENV_FILE" >&2
    set -a; . "$ENV_FILE"; set +a
fi

# The supported env vars: "NAME|description|default". Defaults read from the
# current environment (so an --env file or your shell wins) with a sensible
# fallback. These are exactly the knobs the compositor and its session use.
# Renderer defaults to 'vulkan'. A Vulkan-init failure is a hard error unless the
# GLES fallback is explicitly enabled (COMPOSITOR_RENDERER_FALLBACK=1) — that keeps
# real Vulkan problems visible rather than silently running GLES.
VARS=(
    "COMPOSITOR_RENDERER|Renderer: 'vulkan' (default) or 'gles'|${COMPOSITOR_RENDERER:-vulkan}"
    "COMPOSITOR_RENDERER_FALLBACK|Fall back to GLES if Vulkan init fails (1/gles/true to enable)|${COMPOSITOR_RENDERER_FALLBACK:-}"
    "COMPOSITOR_RENDERER_SYNC|Frame-sync: '' (off), 'infence' (KMS IN_FENCE), or 'kms'|${COMPOSITOR_RENDERER_SYNC:-}"
    "COMPOSITOR_HDR|HDR output (M5): 1 to enable on a PQ-capable display (Vulkan only)|${COMPOSITOR_HDR:-}"
    "COMPOSITOR_DEPTH|Scanout bit depth: 10 for 10-bit/deep-color SDR (no HDR); empty/8 = 8-bit|${COMPOSITOR_DEPTH:-}"
    "COMPOSITOR_VRR|Adaptive sync / VRR: 1/on (default) or 0/off|${COMPOSITOR_VRR:-}"
    "COMPOSITOR_CAPTURE_ENCODER|HW video encoder: 'nvenc' (default), 'vaapi', or 'mesa'|${COMPOSITOR_CAPTURE_ENCODER:-}"
    "Y5_VK_DIAG|Vulkan diagnostics overlay: '' (off), 'vk', or 'blit'|${Y5_VK_DIAG:-}"
    "COMPOSITOR_LOG_LEVEL|y5 log levels, comma-separated: error,warn,info,trace|${COMPOSITOR_LOG_LEVEL:-info,warn,error}"
    "COMPOSITOR_RENDER_NODE|DRM render node (e.g. /dev/dri/renderD129); empty = auto-pick|${COMPOSITOR_RENDER_NODE:-}"
    "COMPOSITOR_DESKTOP_NAME|XDG_CURRENT_DESKTOP advertised to clients; empty = default|${COMPOSITOR_DESKTOP_NAME:-}"
    "COMPOSITOR_WINDOW_CLIENT_SIZE_FALLBACK|Window sizing: 1 = client xdg geometry instead of compositor-tracked|${COMPOSITOR_WINDOW_CLIENT_SIZE_FALLBACK:-}"
    "COMPOSITOR_WINDOW_SUBSURFACE_SHRINKS|Window sizing: 1 = fit the whole surface tree (subsurface can shrink it)|${COMPOSITOR_WINDOW_SUBSURFACE_SHRINKS:-}"
    "WAYLAND_DISPLAY|(winit) host Wayland socket to nest into|${WAYLAND_DISPLAY:-wayland-0}"
    "XDG_RUNTIME_DIR|Runtime dir containing the Wayland socket|${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
    "RUST_BACKTRACE|Rust panic backtrace: 0, 1, or full|${RUST_BACKTRACE:-1}"
    "RUST_LOG|tracing filter for vendored deps (smithay/wgpu); empty = off|${RUST_LOG:-}"
)

if [ "$INTERACTIVE" = 1 ]; then
    echo "== Interactive env setup (Enter keeps the [default]) ==" >&2
fi
for entry in "${VARS[@]}"; do
    IFS='|' read -r name desc def <<<"$entry"
    val="$def"
    if [ "$INTERACTIVE" = 1 ]; then
        printf '\n%s\n  %s\n  [%s] ' "$name" "$desc" "${def:-<unset>}" >&2
        IFS= read -r input </dev/tty || input=""
        [ -n "$input" ] && val="$input"
    fi
    # Export only non-empty values; empty = leave unset so the compositor's own
    # default applies.
    if [ -n "$val" ]; then export "$name=$val"; fi
done

echo "" >&2

# Collapse the individual COMPOSITOR_* knobs into the settings file the compositor reads.
# shellcheck disable=SC1091
. "$HERE/compositor-env.sh"
#compositor_write_settings

BIN="$("$HERE/build.sh" "$BACKEND" "$PROFILE")"
echo ">> running $BIN  [backend=$BACKEND profile=$PROFILE renderer=${COMPOSITOR_RENDERER:-vulkan}]" >&2
exec "$BIN"
