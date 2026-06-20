#!/usr/bin/env bash
# Stop and remove the dev container.
# Usage: ./stop.sh
podman rm -f -t 0 y5-compositor-smithay-dev
