#!/usr/bin/env bash
# Shared by merge-to-main.sh and with-merge-lock.sh: take the merge lock (a directory),
# recording the holder's PID so a lock left behind by a killed holder (SIGKILL skips the
# EXIT trap) is detected and removed instead of blocking every later merge.
#   source scripts/merge-lock.sh; take_merge_lock "$lock" "who" MAX_WAITS
take_merge_lock() {
  local lock=$1 who=$2 max=$3 waited=0 pid
  until mkdir "$lock" 2>/dev/null; do
    pid=$(sed -n 's/.*pid=\([0-9]*\).*/\1/p' "$lock/owner" 2>/dev/null)
    if [ -n "$pid" ] && ! kill -0 "$pid" 2>/dev/null; then
      echo "merge lock: removing a stale lock ($(cat "$lock/owner" 2>/dev/null)): pid $pid is gone" >&2
      rm -rf "$lock"
      continue
    fi
    if [ $waited -eq 0 ]; then echo "waiting for merge lock ($lock): $(cat "$lock/owner" 2>/dev/null)" >&2; fi
    waited=$((waited + 1))
    if [ $waited -gt "$max" ]; then echo "merge lock held for $((max * 5 / 60))+ minutes by $(cat "$lock/owner" 2>/dev/null); ask the coordinator" >&2; return 1; fi
    sleep 5
  done
  echo "$who $(date) pid=$$" > "$lock/owner"
}
