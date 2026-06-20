#!/usr/bin/env bash
# Verify the committed "# --- GENERATED WORKSPACE LINKS ---" blocks are fresh: re-run
# workspace.link.js in every discovered entry that already has a generated block, then
# assert the working tree is unchanged. Fails CI if someone added/renamed a crate without
# re-running link.all.sh (the footgun called out in CLAUDE.md).
#
# Entries without a generated block are skipped (they are not wired into the link system),
# so this never *forces* a block onto a workspace that never had one — it only checks for
# staleness of blocks that already exist.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

cd "$REPO_ROOT"
command -v node >/dev/null 2>&1 || die "node is required for the link-drift check"

# CI containers check out the repo as a different uid than the container user, so git
# treats the work tree as untrusted and refuses it ("dubious ownership", which can surface
# as "not a git repository"). Trust it explicitly. Harmless / idempotent locally.
git config --global --add safe.directory "$REPO_ROOT" 2>/dev/null || true

marker='# --- GENERATED WORKSPACE LINKS START ---'
checked=0

while IFS= read -r entry; do
    if ! grep -qF "$marker" "$entry/Cargo.toml" 2>/dev/null; then
        log "skip $entry (no generated links block)"
        continue
    fi
    log "relink $entry"
    ( cd "$entry" && node "$REPO_ROOT/workspace.link.js" >/dev/null )
    checked=$((checked + 1))
done < <("$(dirname "${BASH_SOURCE[0]}")/discover-workspaces.sh" --lines)

log "re-linked $checked entr(ies); checking for drift"

# Whole-tree diff: this job only runs workspace.link.js (which edits nothing but the
# generated Cargo.toml blocks) on a clean checkout, so any change here IS link drift.
# Avoids a literal '*/Cargo.toml' git pathspec, which older git versions reject.
if ! git diff --quiet; then
    echo "::link-drift:: GENERATED WORKSPACE LINKS are stale. Run ./link.all.sh and commit:" >&2
    git --no-pager diff >&2
    exit 1
fi

log "link-drift OK — all generated link blocks are up to date"
