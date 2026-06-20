#!/usr/bin/env bash
# Frame-callback diagnostic for xwayland-satellite.
#
# Pairs every "[frame] ... request callback=..." line (which carries the
# surface id) with every "[frame] host fired done ... callback=..." line
# (which carries only the callback id), and reports, PER SURFACE, how many
# frame callbacks were requested vs. how many the host (Y5) actually fired.
#
# A surface with requested>0, fired=0 is being starved of frame callbacks by
# the host -> an X11 client throttling on it (Vulkan FIFO present) renders one
# frame then stalls (black).
#
# Usage:
#   ./awk.sh [path-to-log]      (default: /tmp/sat-frame.log)
#
# Capture a log first with, e.g.:
#   RUST_LOG=debug ./target/release/xwayland-satellite :12 \
#       --force-scale 1 --ignore-fractional-scale 2>&1 | tee /tmp/sat-frame.log

set -euo pipefail

LOG="${1:-/tmp/sat-frame.log}"

if [[ ! -f "$LOG" ]]; then
  echo "log not found: $LOG" >&2
  echo "usage: $0 [path-to-log]" >&2
  exit 1
fi

echo "== surface -> window mapping =="
grep -E "associate wl_surface@[0-9]+ with Window|creating toplevel for Window" "$LOG" || true
echo

echo "== buffer commits per surface (does the torn-off window ever draw frame 2?) =="
grep -E "\[buf\].* commit " "$LOG" | grep -oE "wl_surface@[0-9]+" | sort | uniq -c | sort -rn
echo

echo "== buffer attaches per surface =="
grep -E "\[buf\].* attach " "$LOG" | grep -oE "wl_surface@[0-9]+" | sort | uniq -c | sort -rn
echo

echo "== frame callbacks: requested vs fired (per surface) =="
awk '
/\[frame\].*request callback=/ {
  match($0, /wl_surface@[0-9]+/);  surf = substr($0, RSTART, RLENGTH)
  match($0, /wl_callback@[0-9]+/); cb   = substr($0, RSTART, RLENGTH)
  req[surf]++; owner[cb] = surf
}
/\[frame\] host fired done.*callback=/ {
  match($0, /wl_callback@[0-9]+/); cb   = substr($0, RSTART, RLENGTH)
  s = owner[cb]; if (s == "") s = "(unknown)"; fired[s]++
}
END {
  for (s in req)   printf "%-16s requested=%-8d fired=%d\n", s, req[s], fired[s]+0
  for (s in fired) if (!(s in req)) printf "%-16s requested=0        fired=%d\n", s, fired[s]
}' "$LOG" | sort
