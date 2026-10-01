#!/usr/bin/env bash
# Run the full visual suite, keep its scores as a named snapshot and compare
# against an earlier one:
#
#   tools/score_snapshot.sh <label> [<previous label>]
#
# Snapshots live in tests/output/snapshots/<label>.json (gitignored). The font
# index cache is global, so runs here never read or write it: a worktree's
# fonts/ must not leak into, or pick up, another checkout's index.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
label="$1"
prev="${2:-}"
snap="$ROOT/tests/output/snapshots"
mkdir -p "$snap"
export DOCXSIDE_NO_FONT_CACHE=1
# Cargo can reuse a stale test binary after checkouts that keep mtimes.
touch src/lib.rs
./tools/run-tests.sh --test visual_comparison > "$snap/$label.log" 2>&1 || true
cp tests/output/latest_scores.json "$snap/$label.json"
if [ -n "$prev" ]; then
    python3 "$ROOT/tools/compare_scores.py" "$snap/$prev.json" "$snap/$label.json"
fi
