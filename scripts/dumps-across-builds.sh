#!/usr/bin/env bash
# Continuation dumps across builds (0178, TESTING.md "Dumps across builds"): exports a
# format 2 dump at every idle point of the codemode corpus with an older commit's build,
# imports each one with this checkout's build, finishes the run and compares it with the
# older build's uninterrupted run. Exit 1 if a dump does not load or finishes differently.
#
#   scripts/dumps-across-builds.sh OLD_COMMIT
#
# Builds scripts/dumpx/main.rs twice, against `git archive OLD_COMMIT` and against this
# checkout, in target/dumpx/ (the older commit must have the dump API of 0172 or later).
set -euo pipefail
old="${1:?usage: scripts/dumps-across-builds.sh OLD_COMMIT}"
root="$(git rev-parse --show-toplevel)"
work="$root/target/dumpx"
rm -rf "$work/old-src" "$work/dumps"
mkdir -p "$work/old-src"
git -C "$root" archive "$old" | tar -x -C "$work/old-src"

crate() { # crate DIR DJS_ROOT
  mkdir -p "$1/src"
  cp "$root/scripts/dumpx/main.rs" "$1/src/main.rs"
  cat > "$1/Cargo.toml" <<EOT
[package]
name = "dumpx"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
djs-core = { path = "$2/crates/djs-core" }
djs-prelude = { path = "$2/crates/djs-prelude" }

[profile.dev.package.djs-core]
opt-level = 1

[workspace]
EOT
}
crate "$work/old" "$work/old-src"
crate "$work/new" "$root"
for side in old new; do
  (cd "$work/$side" && CARGO_TARGET_DIR="$work/$side/target" cargo build --quiet)
done
echo "== export with $(git -C "$root" rev-parse --short "$old")"
"$work/old/target/debug/dumpx" export "$work/old-src/tests/corpus" "$work/dumps"
echo "== import with $(git -C "$root" rev-parse --short HEAD) (and the working tree)"
"$work/new/target/debug/dumpx" import "$work/dumps"
