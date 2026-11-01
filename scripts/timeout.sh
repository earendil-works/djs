#!/usr/bin/env bash
# Portable timeout (macOS has no `timeout`): scripts/timeout.sh SECONDS command [args...]
# Runs the command in its own process group and kills the whole group on timeout, so
# children (e.g. test binaries spawned by cargo) do not survive as orphans.
# Exit status: the command's, or 142 on timeout.
set -euo pipefail
secs="$1"; shift
exec perl -e '
  my $t = shift;
  my $pid = fork() // die "fork: $!";
  if ($pid == 0) { setpgrp(0, 0); exec @ARGV or die "exec: $!"; }
  $SIG{ALRM} = sub { kill "TERM", -$pid; sleep 2; kill "KILL", -$pid; exit 142 };
  $SIG{INT} = $SIG{TERM} = sub { kill "TERM", -$pid; exit 130 };
  alarm $t;
  waitpid($pid, 0);
  exit($? & 127 ? 128 + ($? & 127) : $? >> 8);
' "$secs" "$@"
