#!/usr/bin/env bash
# Launch a client application inside the running dev container, pointed at the
# compositor's Wayland socket (wayland-1).
# Usage: ./launch.sh [app]   (default: alacritty; "chrome" is a preset)
set -euo pipefail

APP="${1:-alacritty}"
NAME=y5-compositor-smithay-dev

case "$APP" in
    chrome|google-chrome)
        exec podman exec -it -e WAYLAND_DISPLAY=wayland-1 "$NAME" \
            google-chrome-stable --no-sandbox \
            --ozone-platform=wayland --enable-features=UseOzonePlatform
        ;;
    *)
        # Any Wayland client available in the image (alacritty, foot, weston-terminal, ...).
        exec podman exec -it -e WAYLAND_DISPLAY=wayland-1 "$NAME" "$APP"
        ;;
esac
