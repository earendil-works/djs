#!/usr/bin/env bash
# Builds djs-wasm (profile wasm) at a list of commits and prints one TSV row per commit:
# commit, subject, file, code, data, names, stripped, stripped gzip -9 (0052).
# Each commit is exported with `git archive` into a scratch directory and built with one
# shared target dir, so later builds reuse the dependency artifacts. `tar -m` matters:
# cargo hashes path packages relative to the workspace root, so every commit maps to the
# same artifacts, and with the commit timestamps restored older sources look fresh.
#
# usage: scripts/wasm-size-history.sh [--out DIR] COMMIT...
#   DIR defaults to work/scratch/size-history (gitignored); modules are kept there.
set -euo pipefail
cd "$(dirname "$0")/.."
root=$(pwd)
out=work/scratch/size-history
if [ "${1:-}" = "--out" ]; then
  out=$2
  shift 2
fi
mkdir -p "$out"
out=$(cd "$out" && pwd)
export CARGO_TARGET_DIR="$out/target"

section_sizes() {
  python3 - "$1" <<'EOF'
import sys
b = open(sys.argv[1], "rb").read()
def leb(i):
    r = s = 0
    while True:
        x = b[i]; i += 1; r |= (x & 0x7F) << s; s += 7
        if x < 0x80: return r, i
i, code, data, names = 8, 0, 0, 0
while i < len(b):
    sid = b[i]; size, j = leb(i + 1)
    if sid == 10: code = size
    elif sid == 11: data = size
    elif sid == 0:
        n, k = leb(j)
        if b[k:k + n] == b"name": names = size
    i = j + size
print(f"{code}\t{data}\t{names}")
EOF
}

printf 'commit\tsubject\tfile\tcode\tdata\tnames\tstripped\tgzip\n'
for c in "$@"; do
  src="$out/src-$c"
  rm -rf "$src"
  mkdir -p "$src"
  git -C "$root" archive "$c" | tar -x -m -C "$src"
  if ! (cd "$src" && cargo build -p djs-wasm --target wasm32-unknown-unknown --profile wasm --quiet) \
    >"$out/build-$c.log" 2>&1; then
    printf '%s\tbuild failed (see %s)\n' "$c" "$out/build-$c.log"
    continue
  fi
  wasm="$out/$c.wasm"
  cp "$CARGO_TARGET_DIR/wasm32-unknown-unknown/wasm/djs_wasm.wasm" "$wasm"
  cp "$wasm" "$out/$c.stripped.wasm"
  wasm-strip "$out/$c.stripped.wasm"
  subject=$(git -C "$root" log -1 --format=%s "$c" | cut -c1-50)
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$c" "$subject" "$(wc -c <"$wasm" | tr -d ' ')" \
    "$(section_sizes "$wasm")" "$(wc -c <"$out/$c.stripped.wasm" | tr -d ' ')" \
    "$(gzip -9c "$out/$c.stripped.wasm" | wc -c | tr -d ' ')"
  rm -rf "$src"
done
