#!/bin/bash
# base-churn.sh REV... (0164): builds djs-prelude's base image at each commit (a `git
# archive` into a scratch tree, one shared target dir, niced) and prints
# "rev sha256(base[roots..])[:16] length layout-hash", or "rev FAIL" (older commits have no
# djs-prelude). `churn.py --deps` reads the output to know when the builtin prefix changed:
#
#   scripts/snapshot-research/base-churn.sh $(python3 scripts/snapshot-research/churn.py --base-revs) > base.tsv
#   python3 scripts/snapshot-research/churn.py --deps base.tsv
#
# The roots cell is left out: it holds the old build hash or the descriptor, which churn.py
# compares on its own. About 25 s per commit.
set -u
repo=$(git rev-parse --show-toplevel)
W=${DJS_BASE_CHURN_DIR:-$repo/work/scratch/base-churn}
mkdir -p "$W"
for rev in "$@"; do
  rm -rf "$W/src"; mkdir -p "$W/src"
  # -m: fresh mtimes, so cargo rebuilds what changed (archive mtimes are the commit's).
  git -C "$repo" archive "$rev" | tar -x -m -C "$W/src"
  if ! ( cd "$W/src" && CARGO_TARGET_DIR="$W/target" nice -n 15 cargo build -q -j 4 -p djs-prelude >"$W/build-$rev.log" 2>&1 ); then
    echo "$rev FAIL"; continue
  fi
  img=$(ls -t "$W"/target/debug/build/djs-prelude-*/out/base.img | head -1)
  python3 - "$rev" "$img" <<'PY'
import sys, hashlib, struct
rev, img = sys.argv[1], sys.argv[2]
b = open(img, "rb").read()
roots = struct.unpack_from("<I", b, 4)[0]
print(rev, hashlib.sha256(b[roots:]).hexdigest()[:16], len(b), f"{struct.unpack_from('<I', b, 12)[0]:08x}", flush=True)
PY
done
