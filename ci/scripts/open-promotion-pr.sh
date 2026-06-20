#!/usr/bin/env bash
# Open or update the develop -> master promotion PR/MR, using the generated report as its
# body. The request is created in a NON-merged state: master is a protected branch and the
# human approves/merges manually. This script never merges.
#
# Usage: open-promotion-pr.sh [report.md]   (default body: ci/scripts/gen-report.sh output)
#
# Env:
#   PROMOTION_SOURCE  source branch (default: develop)
#   PROMOTION_TARGET  target branch (default: master)

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
cd "$REPO_ROOT"

src="${PROMOTION_SOURCE:-develop}"
dst="${PROMOTION_TARGET:-master}"
title="Promote $src → $dst"

bodyfile="${1:-}"
if [ -z "$bodyfile" ]; then
    bodyfile="$(mktemp)"
    "$(dirname "${BASH_SOURCE[0]}")/gen-report.sh" > "$bodyfile"
fi

if is_github; then
    command -v gh >/dev/null 2>&1 || die "gh CLI required on GitHub"
    if gh pr view "$src" --json number >/dev/null 2>&1; then
        log "updating existing PR $src -> $dst"
        gh pr edit "$src" --body-file "$bodyfile" --title "$title"
    else
        log "creating PR $src -> $dst"
        gh pr create --base "$dst" --head "$src" --title "$title" --body-file "$bodyfile"
    fi

elif is_gitlab; then
    [ -n "${GITLAB_TOKEN:-}" ] || die "GITLAB_TOKEN required on GitLab"
    api="$CI_API_V4_URL/projects/$CI_PROJECT_ID/merge_requests"
    iid="$(curl -sf --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
        "$api?source_branch=$src&target_branch=$dst&state=opened" \
        | grep -o '"iid":[0-9]*' | head -n1 | cut -d: -f2 || true)"
    if [ -n "$iid" ]; then
        log "updating existing MR !$iid"
        curl -sf --request PUT --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
            --data-urlencode "title=$title" \
            --data-urlencode "description=$(cat "$bodyfile")" \
            "$api/$iid" >/dev/null
    else
        log "creating MR $src -> $dst"
        curl -sf --request POST --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
            --data-urlencode "source_branch=$src" \
            --data-urlencode "target_branch=$dst" \
            --data-urlencode "title=$title" \
            --data-urlencode "description=$(cat "$bodyfile")" \
            --data "remove_source_branch=false" \
            "$api" >/dev/null
    fi
else
    die "not running under GitHub or GitLab CI"
fi

log "promotion request ready for manual review: $src -> $dst"
