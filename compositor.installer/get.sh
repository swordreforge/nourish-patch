#!/usr/bin/env bash
# One-line bootstrap for the y5 Wayland compositor (Fedora 44).
#
# This is the hostable script behind the single install command. Host it at a
# stable URL (e.g. https://nourish.snowies.com/install) so users can run:
#
#     curl -fsSL https://nourish.snowies.com/install | bash
#
# It downloads the prebuilt release tarball, verifies its checksum, unpacks it to
# a temp dir, and launches the interactive installer. Because the tarball ships
# compiled binaries, the installer only pulls the runtime shared libraries — no
# Rust toolchain, no -devel headers.
#
# Overridable via env:
#   Y5_RELEASE_URL  full URL to package.tar.gz
#                   (default: the Fedora 44 "latest" release on nourish.snowies.com)
#   Y5_INSTALL_ARGS extra args forwarded to y5-install (e.g. --dry-run)
set -euo pipefail

BASE_URL="${Y5_RELEASE_BASE:-https://nourish.snowies.com/release/latest/fedora44}"
URL="${Y5_RELEASE_URL:-$BASE_URL/package.tar.gz}"
SUMS_URL="${Y5_SUMS_URL:-$BASE_URL/SHA256SUMS}"

say()  { printf '\033[1;36m::\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "missing required tool: $1"; }

need curl
need tar
need sha256sum

# Friendly (non-fatal) Fedora check — the artifact is built for Fedora 44.
if [ -r /etc/os-release ]; then
    . /etc/os-release
    case "${ID:-}" in
        fedora) : ;;
        *) say "warning: this release targets Fedora; detected '${ID:-unknown}'. Continuing." ;;
    esac
fi

WORK="$(mktemp -d "${TMPDIR:-/tmp}/y5-install.XXXXXX")"
cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

say "downloading $URL"
curl -fSL --proto '=https' "$URL" -o "$WORK/package.tar.gz" \
    || die "download failed: $URL"

# Verify the checksum when SHA256SUMS is published alongside the tarball.
if curl -fsSL --proto '=https' "$SUMS_URL" -o "$WORK/SHA256SUMS" 2>/dev/null; then
    say "verifying checksum"
    want="$(awk '{print $1}' "$WORK/SHA256SUMS" | head -n1)"
    have="$(sha256sum "$WORK/package.tar.gz" | awk '{print $1}')"
    [ -n "$want" ] && [ "$want" = "$have" ] || die "checksum mismatch (want $want, have $have)"
else
    say "no SHA256SUMS published; skipping checksum verification"
fi

say "unpacking"
tar -xzf "$WORK/package.tar.gz" -C "$WORK"
STAGE="$WORK/y5-install"
[ -x "$STAGE/y5-install" ] || die "artifact missing the installer (expected $STAGE/y5-install)"

# The installer is interactive. Under `curl ... | bash` this script's stdin is the
# pipe, so feed the installer the real terminal when one exists.
say "launching installer"
export Y5_INSTALL_STAGE="$STAGE"
# shellcheck disable=SC2086 # word-splitting of optional args is intended
if [ -e /dev/tty ]; then
    exec "$STAGE/y5-install" ${Y5_INSTALL_ARGS:-} < /dev/tty
else
    exec "$STAGE/y5-install" ${Y5_INSTALL_ARGS:-}
fi
