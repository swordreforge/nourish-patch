#!/usr/bin/env bash
# Build the dev container image (y5-compositor-smithay-dev) from Containerfile.
# Build context is the repo root so the Containerfile's COPY lines see the workspaces.
# Usage: ./image.sh
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HERE/.." && pwd)"

podman build -t y5-compositor-smithay-dev -f "$HERE/Containerfile" "$REPO_ROOT"
