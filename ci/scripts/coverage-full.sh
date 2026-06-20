#!/usr/bin/env bash
# Full-codebase coverage for ONE workspace entry, including dead code.
#
# Usage: coverage-full.sh <entry-path>     e.g. coverage-full.sh compositor.loader
#
# Two-phase model (matches the "dry run finds everything, then unit tests merge" intent):
#   1. BASELINE (no exec): build + instrument every target (--all-targets) without running
#      anything. LLVM source-based coverage embeds a region for every instrumented line,
#      so functions never executed — including otherwise-dead / feature-gated code — are
#      present at zero counts instead of being omitted from the report.
#   2. UNIT RUN: run the workspace's unit tests, accumulating real hit counts into the
#      same profile directory.
#   3. MERGE -> lcov: cargo-llvm-cov merges the baseline + unit profiles into one lcov,
#      where dead code reads 0% and tested code reads its true percentage.
#
# Per-entry lcov is written to  $REPO_ROOT/.ci-coverage/<slug>.lcov  (slug = entry with
# '/' and '.' turned into '_'). merge-coverage.sh later fuses all entries into one report.
#
# Feature axis: the entry that owns the y5_compositor [[bin]] is additionally built with
# --features udev_release so the DRM/KMS backend code is instrumented too. Override the
# whole feature set with Y5_COV_FEATURES if needed.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

entry="${1:?usage: coverage-full.sh <entry-path>}"
[ -d "$REPO_ROOT/$entry" ] || die "no such entry: $entry"
command -v cargo-llvm-cov >/dev/null 2>&1 || cargo llvm-cov --version >/dev/null 2>&1 \
    || die "cargo-llvm-cov is required (cargo install cargo-llvm-cov)"

slug="$(printf '%s' "$entry" | tr '/.' '__')"
outdir="$REPO_ROOT/.ci-coverage"
mkdir -p "$outdir"
out="$outdir/$slug.lcov"

# Feature args: udev_release for the entry that holds the y5_compositor bin.
feature_args=()
if [ -n "${Y5_COV_FEATURES:-}" ]; then
    # shellcheck disable=SC2206
    feature_args=(--features "$Y5_COV_FEATURES")
else
    bin_dir="$(y5_bin_crate_dir)"
    bin_ws="$(y5_workspace_root_of "$REPO_ROOT/$bin_dir")"
    if [ "$bin_ws" = "$REPO_ROOT/$entry" ]; then
        feature_args=(--features udev_release)
        log "$entry owns y5_compositor -> instrumenting udev_release backend too"
    fi
fi

cd "$REPO_ROOT/$entry"

log "[$entry] clean coverage profile"
cargo llvm-cov clean --workspace

log "[$entry] phase 1: baseline (instrument all targets, no exec)"
cargo llvm-cov --no-report --no-run --all-targets "${feature_args[@]}"

log "[$entry] phase 2: run unit tests"
cargo llvm-cov --no-report --all-targets "${feature_args[@]}"

log "[$entry] phase 3: merge -> $out"
cargo llvm-cov report --lcov --output-path "$out"

log "[$entry] coverage lcov written: $out"
