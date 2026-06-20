#!/usr/bin/env bash
# LLM doc/README review: given the change set in a PR/MR, ask Claude which docs are now
# stale or missing and post the suggestions as a PR/MR comment. Advisory and comment-only
# — it NEVER edits or commits files.
#
# Skips gracefully (exit 0) when ANTHROPIC_API_KEY or the `claude` CLI is absent, so the
# job stays non-blocking. Writes the suggestions to .ci-report/doc-suggestions.md too, so
# gen-report.sh can fold them into the promotion report.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

cd "$REPO_ROOT"
mkdir -p .ci-report
out=".ci-report/doc-suggestions.md"

if [ -z "${ANTHROPIC_API_KEY:-}" ] || ! command -v claude >/dev/null 2>&1; then
    log "ANTHROPIC_API_KEY or claude CLI missing — skipping doc review (advisory)"
    echo "_doc review skipped (no ANTHROPIC_API_KEY / claude CLI)_" > "$out"
    exit 0
fi

# Resolve the base branch for the diff on either platform.
base=""
if is_github;  then base="${GITHUB_BASE_REF:-master}"
elif is_gitlab; then base="${CI_MERGE_REQUEST_TARGET_BRANCH_NAME:-master}"
else base="${1:-master}"; fi

git fetch -q origin "$base" 2>/dev/null || true
diff="$(git diff "origin/$base...HEAD" 2>/dev/null || git diff "$base...HEAD" 2>/dev/null || true)"
if [ -z "$diff" ]; then
    log "empty diff vs $base — nothing to review"
    echo "_no changes to review against \`$base\`_" > "$out"
    exit 0
fi

# Trim very large diffs to keep the prompt bounded.
diff="$(printf '%s' "$diff" | head -c 120000)"

read -r -d '' prompt <<EOF || true
You are reviewing a change set for the y5 Wayland compositor. Below is the git diff and
the current top-level docs. Identify documentation that the diff makes stale, missing, or
inconsistent — focus on README.md, CLAUDE.md, document/*.md and environment/README.md.

Output GitHub-flavored markdown: a short bulleted list of concrete, specific suggested
edits (which file, which section, what to change). If everything is already accurate,
reply with a single line: "No documentation changes needed." Do not restate the diff.

=== GIT DIFF (vs $base) ===
$diff

=== CLAUDE.md ===
$(head -c 16000 CLAUDE.md 2>/dev/null)
EOF

log "running claude doc review (model claude-opus-4-8)"
if ! claude -p "$prompt" --model claude-opus-4-8 > "$out" 2>/dev/null; then
    log "claude invocation failed — emitting placeholder (advisory)"
    echo "_doc review unavailable (claude invocation failed)_" > "$out"
    exit 0
fi

body="$(printf '### 📝 Documentation suggestions\n\n%s\n\n_— automated, advisory; review before applying._' "$(cat "$out")")"

if is_github && command -v gh >/dev/null 2>&1; then
    gh pr comment "${GITHUB_PR_NUMBER:-}" --body "$body" 2>/dev/null \
        || gh pr comment --body "$body" 2>/dev/null \
        || log "gh pr comment failed (advisory)"
elif is_gitlab && [ -n "${GITLAB_TOKEN:-}" ] && [ -n "${CI_MERGE_REQUEST_IID:-}" ]; then
    curl -sf --request POST \
        --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
        --data-urlencode "body=$body" \
        "$CI_API_V4_URL/projects/$CI_PROJECT_ID/merge_requests/$CI_MERGE_REQUEST_IID/notes" \
        >/dev/null || log "GitLab MR note failed (advisory)"
else
    log "no PR/MR context to comment on — suggestions saved to $out"
fi

log "doc review done -> $out"
