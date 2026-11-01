#!/usr/bin/env bash
# Portable timeout (macOS has no `timeout`): scripts/timeout.sh SECONDS command [args...]
# Runs the command in its own process group and kills the whole group on timeout, so
# children (e.g. test binaries spawned by cargo) do not survive as orphans.
# Exit status: the command's, or 142 on timeout.
#
# The command runs under `nice -n 10`, so agents' test and build runs yield to the gate,
# which is the serial bottleneck of every merge (D1793). Not for the gate itself: when
# DJS_GATE=1 (gate.sh sets it) or the command is merge-to-main.sh, gate.sh or
# with-merge-lock.sh (a niced process cannot lower its niceness again).
set -euo pipefail
secs="$1"; shift
nice_cmd=(nice -n 10)
case "$(basename "$1")" in
  merge-to-main.sh|gate.sh|with-merge-lock.sh) nice_cmd=() ;;
esac
if [ "${DJS_GATE:-}" = 1 ]; then nice_cmd=(); fi
exec perl -e '
  my $t = shift;
  my $pid = fork() // die "fork: $!";
  if ($pid == 0) { setpgrp(0, 0); exec @ARGV or die "exec: $!"; }
  $SIG{ALRM} = sub { kill "TERM", -$pid; sleep 2; kill "KILL", -$pid; exit 142 };
  $SIG{INT} = $SIG{TERM} = sub { kill "TERM", -$pid; exit 130 };
  alarm $t;
  waitpid($pid, 0);
  exit($? & 127 ? 128 + ($? & 127) : $? >> 8);
' "$secs" ${nice_cmd[@]+"${nice_cmd[@]}"} "$@"
