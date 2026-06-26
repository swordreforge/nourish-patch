#!/usr/bin/env bash
# (A) Open a shell on a target distro (nested under the host's Wayland session), with the winit
# y5_compositor compiled-on-that-distro and an `exec.sh` ready to launch it. You land in a shell
# so you can pre-check the distro (libs, GPU, `ldd /usr/local/bin/y5_compositor`, `vulkaninfo`,
# ...); type `exec.sh` to write settings + start the compositor.
#
# Usage: ./run.sh <distro> [debug|release]   (default profile: debug)
#
# Differs from ../run.sh on purpose:
#   * no --network flag (removed)
#   * the source is git-cloned into the image, not mounted/copied — so this runs the
#     binary that was actually COMPILED ON THAT DISTRO. Re-run image.sh after code changes.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
. "$HERE/common.sh"

DISTRO="${1:?usage: run.sh <distro> [debug|release]  (have: $(distro_list | tr '\n' ' ')) }"
PROFILE="${2:-debug}"
distro_validate "$DISTRO"

IMAGE="$(distro_image "$DISTRO" "$PROFILE")"
NAME="$(distro_container "$DISTRO")"

podman image exists "$IMAGE" || "$HERE/image.sh" "$DISTRO" "$PROFILE"

podman rm -f -t 0 "$NAME" 2>/dev/null || true

# Honor a host-set log level; default to the usual dev verbosity otherwise.
LOG_LEVEL="${COMPOSITOR_LOG_LEVEL:-info,warn,error,trace}"

echo ">> shell on $DISTRO ($PROFILE), nested under $WAYLAND_DISPLAY." >&2
echo "   compositor: /usr/local/bin/y5_compositor — run 'exec.sh' to write settings + launch it." >&2
echo "   ('exit' or Ctrl-D leaves the shell and removes the container.)" >&2
# Mounts:
#   - the host Wayland socket (so the nested compositor can connect)
#   - exec.sh into PATH (launches the compositor on demand)
#   - the LIVE settings-writer over the image's copy, so settings fixes apply without a rebuild
podman run -it --rm \
    --name "$NAME" \
    --init \
    --env-file "$ENV_CONTAINER" \
    -e COMPOSITOR_LOG_LEVEL="$LOG_LEVEL" \
    --device nvidia.com/gpu=all \
    -v "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY:/tmp/wayland-host:ro" \
    -v "$HERE/exec.sh:/usr/local/bin/exec.sh:ro" \
    -v "$REPO_ROOT/environment/compositor-env.sh:/working.directory/environment/compositor-env.sh:ro" \
    --security-opt label=disable \
    -w /working.directory \
    "$IMAGE" /bin/bash
