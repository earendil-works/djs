#!/bin/sh
# Merge points of issues on main, oldest first: for each issue NNNN, the last commit on
# main whose subject starts with "NNNN:" (main is linear: every issue lands by
# fast-forward, so its last commit was main's tip once). Prints "pos sha issue".
# Usage: points.sh [first-issue]   (default 0100)
first=${1:-0100}
git log --reverse --format='%h %s' main | awk -v first="$first" '
  { n=$2; if (n ~ /^[0-9][0-9][0-9][0-9]:$/) { sub(":","",n); if (n >= first) last[n]=NR" "$1 } }
  END { for (k in last) print last[k], k }' | sort -n
