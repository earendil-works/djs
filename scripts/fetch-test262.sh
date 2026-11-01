#!/usr/bin/env bash
# Fetch test262 at the pinned commit (D003) into vendor/test262 of the primary checkout.
#
# Worktrees share that one copy. The primary checkout is the parent of git's common
# directory. Set DJS_TEST262 to use a different location.
#
# Idempotent: if the checkout already exists at the pinned commit it does nothing; if it
# exists at another commit it fetches and checks out the pinned one. Only the pinned
# commit is fetched (depth 1), which GitHub allows for any reachable SHA.
set -euo pipefail

COMMIT=7ab7fafa0003f73fc85c1b95d88094d33f7eb8bd
URL=https://github.com/tc39/test262.git

if [ -n "${DJS_TEST262:-}" ]; then
  dest=$DJS_TEST262
else
  common=$(git -C "$(dirname "$0")" rev-parse --git-common-dir)
  case "$common" in
    /*) ;;
    *) common="$(cd "$(dirname "$0")" && cd "$common" && pwd)" ;;
  esac
  dest="$(cd "$common/.." && pwd)/vendor/test262"
fi

if [ -e "$dest/.git" ]; then
  have=$(git -C "$dest" rev-parse HEAD 2>/dev/null || echo none)
  if [ "$have" = "$COMMIT" ]; then
    if [ -n "$(git -C "$dest" status --porcelain --untracked-files=no)" ]; then
      echo "fetch-test262: $dest is at $COMMIT but has local modifications" >&2
      echo "fetch-test262: run 'git -C $dest checkout -- .' to reset it" >&2
      exit 1
    fi
    echo "fetch-test262: $dest is at $COMMIT"
    exit 0
  fi
  echo "fetch-test262: $dest is at $have, moving to $COMMIT"
else
  echo "fetch-test262: cloning test262 into $dest"
  mkdir -p "$dest"
  git -C "$dest" init -q
  git -C "$dest" remote add origin "$URL"
fi

git -C "$dest" fetch -q --depth 1 origin "$COMMIT"
git -C "$dest" -c advice.detachedHead=false checkout -q --force "$COMMIT"

have=$(git -C "$dest" rev-parse HEAD)
if [ "$have" != "$COMMIT" ]; then
  echo "fetch-test262: checkout ended at $have, expected $COMMIT" >&2
  exit 1
fi
echo "fetch-test262: $dest is at $COMMIT"
