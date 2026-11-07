#!/usr/bin/env bash
# The cross-build check for Rust hosts (TESTING.md "Cross builds of djs-embed", 0246): builds
# djs-embed's example crates/djs-embed/examples/cross_check.rs for the targets a host such
# as Gondolin ships for, runs each binary where this machine can (darwin natively and
# x86_64 under Rosetta, linux-musl in docker), and checks that they all print the same
# lines, including that the base image each binary embeds is the one its target makes
# itself. Also lists the runtime crates `cargo tree -e normal` gives for djs-embed.
#
# Not a gate leg: it needs zig 0.17 (the linux-musl linker, scripts/cross/zigcc), the
# rustup targets and docker, which the gate machine need not have. A manual check, run when
# something that could make builds target dependent changes (djs-prelude's build script,
# djs-libm's arch code, the image format).
#
# usage: scripts/cross-check.sh [--release] [--consumer REV] [--targets "T1 T2 ..."]
#   --release        build with the release profile (default: dev)
#   --consumer REV   build a scratch consumer outside the workspace that depends on
#                    djs-embed by `git = "file://<this repository>"`, `rev = REV`, instead
#                    of the in-workspace example (work/scratch/cross-check/consumer)
#   --targets LIST   the targets (default: the four below)
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
CROSS=$ROOT/scripts/cross
TARGETS="aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-musl x86_64-unknown-linux-musl"
profile=dev
rev=
while [ $# -gt 0 ]; do
  case "$1" in
    --release) profile=release; shift ;;
    --consumer) rev=$2; shift 2 ;;
    --targets) TARGETS=$2; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
outdir=debug
[ "$profile" = release ] && outdir=release

# The cross toolchain of a target: zig cc as linker (and C compiler, should a dependency
# ever compile C) for linux-musl; darwin targets need nothing beyond Xcode.
cross_env() {
  local target=$1 zigtarget upper lower
  case "$target" in
    aarch64-unknown-linux-musl) zigtarget=aarch64-linux-musl ;;
    x86_64-unknown-linux-musl) zigtarget=x86_64-linux-musl ;;
    *-apple-darwin) return 0 ;;
    *) echo "unsupported target $target" >&2; return 1 ;;
  esac
  command -v zig >/dev/null || { echo "zig (0.17) is required for $target builds" >&2; return 1; }
  upper=$(echo "$target" | tr 'a-z-' 'A-Z_')
  lower=${target//-/_}
  export "CC_$lower=$CROSS/zigcc-$zigtarget"
  export "AR_$lower=$CROSS/zigar"
  export "CARGO_TARGET_${upper}_LINKER=$CROSS/zigcc-$zigtarget"
}

scratch=$ROOT/work/scratch/cross-check
mkdir -p "$scratch"

# The project to build: the workspace's example, or a consumer by git revision.
if [ -n "$rev" ]; then
  # The repository itself, not a worktree's checkout: git clones from the common dir.
  repo=$(cd "$ROOT" && cd "$(git rev-parse --git-common-dir)" && pwd)
  proj=$scratch/consumer
  mkdir -p "$proj/src"
  cp "$ROOT/crates/djs-embed/examples/cross_check.rs" "$proj/src/main.rs"
  cat > "$proj/Cargo.toml" <<EOF
[package]
name = "djs-cross-check"
version = "0.0.0"
edition = "2024"
publish = false

# Its own workspace: not a member of the repository's.
[workspace]

[dependencies]
djs-embed = { git = "file://$repo", rev = "$rev" }
# The example compares the embedded base with one it makes, through djs-core directly.
djs-core = { git = "file://$repo", rev = "$rev", default-features = false }
djs-prelude = { git = "file://$repo", rev = "$rev", default-features = false, features = ["raw"] }
EOF
  rm -f "$proj/Cargo.lock"
  build() { (cd "$proj" && cargo build -q --profile "$profile" --target "$1"); }
  binary() { echo "$proj/target/$1/$outdir/djs-cross-check"; }
  tree() { (cd "$proj" && cargo tree -q -e normal --prefix none --target "$1" -p djs-embed); }
else
  build() { (cd "$ROOT" && cargo build -q -p djs-embed --example cross_check --profile "$profile" --target "$1"); }
  binary() { echo "$ROOT/target/$1/$outdir/examples/cross_check"; }
  tree() { (cd "$ROOT" && cargo tree -q -e normal --prefix none --target "$1" -p djs-embed); }
fi

# Runs a target's binary on this machine, or says why it cannot.
run() { # target binary
  local target=$1 bin=$2 dir name
  dir=$(dirname "$bin")
  name=$(basename "$bin")
  case "$target" in
    *-apple-darwin)
      [ "$(uname -s)" = Darwin ] || { echo "skip: not on macOS"; return 2; }
      if [ "$target" = x86_64-apple-darwin ] && [ "$(uname -m)" = arm64 ]; then
        arch -x86_64 /usr/bin/true 2>/dev/null || { echo "skip: no Rosetta"; return 2; }
        arch -x86_64 "$bin"
      else
        "$bin"
      fi ;;
    *-linux-musl)
      docker info >/dev/null 2>&1 || { echo "skip: docker is not running"; return 2; }
      local platform=linux/arm64
      [ "${target%%-*}" = x86_64 ] && platform=linux/amd64
      docker run --rm --platform "$platform" -v "$dir:/w:ro" alpine:3 "/w/$name" ;;
  esac
}

status=0
first=
for t in $TARGETS; do
  echo "== $t"
  rustup target list --installed 2>/dev/null | grep -qx "$t" \
    || { echo "missing rust target: rustup target add $t" >&2; status=1; continue; }
  if ! (cross_env "$t" && build "$t"); then
    echo "BUILD FAILED"; status=1; continue
  fi
  bin=$(binary "$t")
  echo "binary: $(du -k "$bin" | cut -f1) KiB, $(file -b "$bin" | cut -d, -f1-2)$(file -b "$bin" | grep -o ', [a-z-]* linked' || true)"
  echo "runtime crates: $(tree "$t" | sed 's/ (.*//' | sort -u | tr '\n' ' ')"
  out=$scratch/$t.txt
  set +e
  # stdout only: docker's pull messages go to stderr.
  run "$t" "$bin" > "$out" 2> "$out.err"
  rc=$?
  set -e
  if [ $rc = 2 ] && grep -q '^skip:' "$out"; then
    cat "$out"; continue
  fi
  tail -n 2 "$out"
  if [ $rc != 0 ]; then
    echo "RUN FAILED (exit $rc): $out"; tail -n 5 "$out.err"; status=1; continue
  fi
  if [ -z "$first" ]; then
    first=$t
  elif ! cmp -s "$scratch/$first.txt" "$out"; then
    echo "OUTPUT DIFFERS from $first:"; diff "$scratch/$first.txt" "$out" | head -20; status=1
  else
    echo "output: same as $first"
  fi
done
[ $status = 0 ] && echo "cross-check: ok" || echo "cross-check: FAILED"
exit $status
