#!/usr/bin/env bash
# Portable timeout (macOS has no `timeout`): scripts/timeout.sh SECONDS command [args...]
# Sends SIGALRM to the command after SECONDS (exit status 142 on timeout).
set -euo pipefail
secs="$1"; shift
exec perl -e 'alarm shift; exec @ARGV or die "exec: $!"' "$secs" "$@"
