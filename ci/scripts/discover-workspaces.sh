#!/usr/bin/env bash
# Discover y5 "workspace entries": directories (depth <= 2 under the repo root, excluding
# target/) that contain BOTH a Cargo.toml and a link.json. That pair is the repo's
# definition of an independently linkable Cargo workspace. Nothing is hardcoded, so the
# set tracks crate/workspace renames automatically — same philosophy as link.all.sh.
#
# Output (stdout):
#   default     compact JSON array, e.g. ["compositor","compositor.loader",...]
#               (consumed by the GitHub matrix via fromJson and by gen-child-pipeline.sh)
#   --lines     one entry path per line (for shell `while read` loops)
#
# Paths are relative to the repo root.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

cd "$REPO_ROOT"

mode="${1:-json}"

entries=()
while IFS= read -r lj; do
    dir="$(dirname "$lj")"
    dir="${dir#./}"
    [ -f "$dir/Cargo.toml" ] || continue
    entries+=("$dir")
done < <(find . -maxdepth 3 -name link.json -not -path '*/target/*' | sort)

[ "${#entries[@]}" -gt 0 ] || die "no workspace entries found (Cargo.toml + link.json)"

case "$mode" in
    --lines)
        printf '%s\n' "${entries[@]}"
        ;;
    json)
        # Build a JSON array by hand (entry paths are simple, no escaping needed) so the
        # script has zero runtime dependencies.
        out="["; sep=""
        for e in "${entries[@]}"; do out+="$sep\"$e\""; sep=","; done
        out+="]"
        printf '%s\n' "$out"
        ;;
    *)
        die "usage: discover-workspaces.sh [--lines|json]"
        ;;
esac
