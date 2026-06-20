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

if ! git diff --quiet -- '*/Cargo.toml' 'Cargo.toml'; then
    echo "::link-drift:: GENERATED WORKSPACE LINKS are stale. Run ./link.all.sh and commit:" >&2
    git --no-pager diff -- '*/Cargo.toml' 'Cargo.toml' >&2
    exit 1
fi

log "link-drift OK — all generated link blocks are up to date"
