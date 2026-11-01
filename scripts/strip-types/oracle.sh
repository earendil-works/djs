#!/usr/bin/env bash
# The strip-types oracle (TESTING.md "TypeScript"): `djs strip-types` must match
# ts-blank-space byte for byte on every corpus file ts-blank-space accepts.
#
#   scripts/strip-types/oracle.sh [path-filter]
#
# Fetches the oracle and corpus on first use (fetch.sh), runs ts-blank-space once into
# work/scratch/strip-types/expected, then djs on every accepted file. Prints the match
# rate and the failures (first differing line); details go to
# work/scratch/strip-types/report.txt.
set -euo pipefail
cd "$(dirname "$0")/../.."
vendor=$(scripts/strip-types/fetch.sh | tail -1)
corpus="$vendor/corpus"
scratch=work/scratch/strip-types
filter="${1:-}"
cargo build -q --release -p djs-cli
djs=target/release/djs

if [ ! -d "$scratch/expected" ]; then
  mkdir -p "$scratch/expected"
  find "$corpus" -name '*.ts' ! -name '*.d.ts' -print0 \
    | STRIP_NODE_DIR="$vendor/node" xargs -0 node scripts/strip-types/blank.mjs \
        "$scratch/expected" "$corpus" > "$scratch/unsupported.txt"
fi

total=0; pass=0
: > "$scratch/report.txt"
while IFS= read -r -d '' exp; do
  rel=${exp#"$scratch/expected/"}
  case "$rel" in *"$filter"*) ;; *) continue ;; esac
  total=$((total + 1))
  if out=$(scripts/timeout.sh 60 "$djs" strip-types "$corpus/$rel" 2>&1) \
      && [ "$out" == "$(cat "$exp")" ]; then
    pass=$((pass + 1))
  else
    {
      echo "== $rel"
      if [[ "$out" == *SyntaxError* ]] && [ "$(printf '%s' "$out" | wc -l)" -lt 2 ]; then
        echo "$out"
      else
        diff <(printf '%s\n' "$out") <(cat "$exp"; echo) | head -6 || true
      fi
    } >> "$scratch/report.txt"
  fi
done < <(find "$scratch/expected" -name '*.ts' -print0 | sort -z)
unsupported=$(wc -l < "$scratch/unsupported.txt" | tr -d ' ')
echo "strip-types: $pass/$total files match ts-blank-space ($unsupported more files are syntax ts-blank-space does not erase)"
grep -c '^== ' "$scratch/report.txt" >/dev/null && grep -h 'SyntaxError' "$scratch/report.txt" \
  | sed 's/.*SyntaxError: //' | sort | uniq -c | sort -rn | head -15 || true
