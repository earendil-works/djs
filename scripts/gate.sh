#!/usr/bin/env bash
# The merge gate: every commit main fast-forwards to has passed it (merge-to-main.sh runs it
# in the gate worktree). The legs, their builds, the input classes that cache them and the
# full run are in scripts/gate.py (TESTING.md "The gate", D2660-D2669).
#
#   scripts/gate.sh               every leg whose inputs have not passed before
#   scripts/gate.sh --full        every leg, cache unread, plus the full-only legs
#   scripts/gate.sh --no-cache    every leg, cache unread
#   scripts/gate.sh --only A,B    only these legs (and their builds)
#   scripts/gate.sh --list        the legs and builds
#   scripts/gate.sh --clear-cache empty the cache (work/scratch/gate-cache/)
set -euo pipefail
cd "$(dirname "$0")/.."
# Not niced by scripts/timeout.sh: everything else yields to the gate (D1793).
export DJS_GATE=1
exec python3 scripts/gate.py "$@"
