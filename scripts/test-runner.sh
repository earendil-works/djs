#!/usr/bin/env bash
# Cargo runner for the gate's `cargo test` (0079): records each test's time.
#
#   CARGO_TARGET_<TRIPLE>_RUNNER=scripts/test-runner.sh cargo test --workspace --quiet
#
# Test binaries (target/*/deps/*) run with libtest's `--report-time`, which stable Rust
# only allows with RUSTC_BOOTSTRAP=1 at run time; the build is not affected, since only
# the runner sets it. Their output is kept in a file and shown only when the binary fails
# (the gate runs quiet). Every `test NAME ... ok <SECS>s>` line is appended to
# $DJS_TEST_TIMES as `SECS<TAB>BINARY<TAB>NAME`. Other programs (`cargo run`) run as is.
set -uo pipefail
bin="$1"; shift
case "$bin" in
  */deps/*) ;;
  *) exec "$bin" "$@" ;;
esac
args=()
for a in "$@"; do
  case "$a" in
    -q|--quiet) ;;              # terse output has no test names
    *) args+=("$a") ;;
  esac
done
out=$(mktemp)
RUSTC_BOOTSTRAP=1 "$bin" ${args[@]+"${args[@]}"} -Z unstable-options --report-time > "$out" 2>&1
rc=$?
if [ -n "${DJS_TEST_TIMES:-}" ]; then
  name=$(basename "$bin" | sed -E 's/-[0-9a-f]{16}$//')
  sed -nE 's/^test (.*) \.\.\. ok <([0-9.]+)s>$/\2\t'"$name"'\t\1/p' "$out" >> "$DJS_TEST_TIMES"
fi
if [ $rc -ne 0 ]; then cat "$out"; else grep -E '^test result:' "$out" | grep -v ' 0 passed; 0 failed; 0 ignored' || true; fi
rm -f "$out"
exit $rc
