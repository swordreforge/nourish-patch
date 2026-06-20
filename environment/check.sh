#!/usr/bin/env bash
# Repo-wide check: workspace conformance lint (layout/chain/naming/size/flat),
# then `cargo check` in every workspace root. Mirrors the gate build.sh applies.
set -euo pipefail

SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${Y5_REPO_ROOT:-}"
if [ -z "$REPO_ROOT" ]; then
    d="$SELF_DIR"
    while [ "$d" != "/" ]; do
        if compgen -G "$d/compositor*" >/dev/null 2>&1 && [ -f "$d/workspace.lint.js" ]; then REPO_ROOT="$d"; break; fi
        d="$(dirname "$d")"
    done
fi
[ -n "$REPO_ROOT" ] || { echo "check.sh: could not locate repo root" >&2; exit 1; }

echo ">> workspace.lint" >&2
( cd "$REPO_ROOT" && node workspace.lint.js >&2 )

# Every dir holding a [workspace] Cargo.toml, including roots inside containers.
fail=0
while IFS= read -r ws; do
    ws_dir="$(dirname "$ws")"
    echo ">> cargo check: ${ws_dir#"$REPO_ROOT"/}" >&2
    ( cd "$ws_dir" && cargo check --quiet </dev/null ) || fail=1
done < <(grep -l '^\[workspace\]' "$REPO_ROOT"/compositor*/Cargo.toml "$REPO_ROOT"/compositor*/*/Cargo.toml 2>/dev/null)
exit $fail
