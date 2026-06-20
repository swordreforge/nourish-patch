#!/usr/bin/env bash
# CD step: build the release binary and bundle the downloadable artifacts. This is the
# real "deploy" for y5 — produce versioned, checksummed artifacts; live host deploy is a
# separate, optional, manual job (environment/build-release.sh).
#
# Usage: package-release.sh [version]
#   version defaults to the git tag (CI_COMMIT_TAG / GITHUB_REF_NAME) or the short SHA.
#
# Reuses environment/build.sh verbatim (it discovers the entry crate + workspace itself).
# Output: dist/y5-compositor-<version>.tar.gz + dist/SHA256SUMS, paths printed to stdout.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
cd "$REPO_ROOT"

version="${1:-${CI_COMMIT_TAG:-${GITHUB_REF_NAME:-$(git rev-parse --short HEAD)}}}"
dist="$REPO_ROOT/dist"
stage="$dist/y5-compositor-$version"
rm -rf "$dist"; mkdir -p "$stage"

log "building udev release binary (via environment/build.sh)"
udev_bin="$(environment/build.sh udev release)"
cp "$udev_bin" "$stage/y5_compositor"

log "building winit release binary"
winit_bin="$(environment/build.sh winit release)"
cp "$winit_bin" "$stage/y5_compositor.winit"

# Fold in docs + coverage if earlier stages produced them (optional).
[ -d "$REPO_ROOT/public" ] && cp -r "$REPO_ROOT/public" "$stage/docs"
[ -f "$REPO_ROOT/.ci-coverage/coverage.lcov" ] && cp "$REPO_ROOT/.ci-coverage/coverage.lcov" "$stage/"
[ -f "$REPO_ROOT/.ci-coverage/coverage.txt" ]  && cp "$REPO_ROOT/.ci-coverage/coverage.txt"  "$stage/"

cat > "$stage/RELEASE.txt" <<EOF
y5_compositor release $version
commit: $(git rev-parse HEAD)
built:  $(git log -1 --format=%cI HEAD)
backends: udev (y5_compositor), winit (y5_compositor.winit)
EOF

tarball="$dist/y5-compositor-$version.tar.gz"
log "packaging $tarball"
tar -C "$dist" -czf "$tarball" "y5-compositor-$version"

( cd "$dist" && sha256sum "$(basename "$tarball")" > SHA256SUMS )

log "release artifacts:"
printf '%s\n' "$tarball" "$dist/SHA256SUMS"
