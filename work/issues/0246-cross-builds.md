# 0246: djs-embed for Gondolin's targets: cross-builds, image portability, consuming it

status: done
agent: libm
branch: issue/0246-cross-builds
depends-on: 0240, 0245
touches: scripts/ (a cross-build check), crates/djs-embed (README / docs on depending on it), crates/djs-prelude (only if its build script needs a change), TESTING.md, docs/guide/rust-api.md
guide: docs/guide/rust-api.md

## Goal

Gondolin builds `gondolind` for `aarch64-unknown-linux-musl` and `x86_64-unknown-linux-musl`
with `zig cc` wrappers (`~/Development/gondolin/native/scripts/cross-env.sh`, `zigcc*`,
read-only: copy the approach, not the files' paths) and for `aarch64/x86_64-apple-darwin`. The
research (djs-hooks.md E1) flagged as untested that djs-prelude's build script makes the base
image on the build host: check that images are target independent for 64-bit little-endian
targets (the base image a cross build embeds must be the one the target would make itself).

- A script (e.g. scripts/cross-check.sh) that builds djs-embed and a small example binary
  (run a script, call a host function, async entry) for the four targets, and runs them where
  possible: darwin natively and x86_64-apple-darwin under Rosetta; linux-musl binaries in docker
  (`docker run --rm -v ...:/w alpine /w/bin`; x86_64 via `--platform linux/amd64`). Compare the
  embedded base image bytes across targets (the example can print its hash).
- Document how Gondolin depends on it: a path dependency for local development and a git
  dependency pinned to a revision (`git = "https://github.com/earendil-works/djs", rev = ...`),
  with the features to use (`default-features` choices: no `dump`, `lazy`, `compile-cache`
  unless wanted); check that a git dependency builds (files outside a crate directory that
  build scripts or `include_str!` read: builtins/gen/prelude.js, tests/builtins/registry.tsv;
  the registry feature should be off for embedders, or the path must resolve from a checkout).
  Verify with a scratch cargo project outside the workspace that depends on the local repo by
  `git = "file://..."` and a rev.
- Count the runtime crates `cargo tree -e normal` gives for djs-embed (should be djs-embed,
  djs-core, djs-prelude, djs-libm: all djs crates).
- Not a gate leg if it needs docker/zig (they are not guaranteed on the gate machine); a
  documented manual check, run once here, with results in the issue.

## Acceptance criteria
- [x] cross-build script; all four targets build; binaries run in docker / Rosetta with the same
      output; base image identical across targets (or the reason it isn't, and a fix)
- [x] docs on depending on djs-embed by git rev; a scratch consumer builds by `git = file://`
- [x] runtime crate list recorded

## Log

- 2026-11-07 (libm): `crates/djs-embed/examples/cross_check.rs`: a script with a sync
  (`log`) and an async (`fetchText`) host function through an async entry; prints the
  result, numbers whose bits depend on djs-libm and the number formatting, fuel used, the
  snapshot's size, and compares `djs_prelude::BASE` (made by the build machine's build
  script) with a base the binary makes at run time (`Vm::new` + `prepare_base_image`):
  exit 1 if they differ.
- `scripts/cross-check.sh` (+ `scripts/cross/zigcc*`, `zigar`: Gondolin's zig cc wrapper
  approach, copied) builds it for aarch64/x86_64-apple-darwin and -unknown-linux-musl, runs
  darwin natively / `arch -x86_64` (Rosetta), linux in `docker run --platform linux/arm64|amd64
  alpine:3`, compares stdout across targets, lists the runtime crates per target.
  `--release`, `--consumer REV` (a scratch project in work/scratch/cross-check/consumer with
  its own `[workspace]`, deps by `git = "file://<common git dir>"`, `rev`; release profile
  like Gondolin's: fat LTO, 1 CGU, strip).
- Results (all `cross-check: ok`, every target's output identical, base 188,672 bytes, fnv
  c1d1c3d1f74dfda4, "base made here: identical" on all four):
  - workspace, dev: darwin arm64 4.3 MB, x86_64 4.3 MB; musl aarch64 42 MB, x86_64 43 MB
    (static, debug info); ~2 min cold.
  - workspace, release: 3.1 / 3.1 / 19.9 / 19.1 MB.
  - consumer by git rev b947a8bb, dev: builds; release with fat LTO + strip: 2.4 / 2.4 /
    2.4 / 2.5 MB (static musl).
  - runtime crates (`cargo tree -e normal -p djs-embed`, every target, workspace and
    consumer): djs-core, djs-embed, djs-libm, djs-prelude. No third-party crate.
- Why images are target independent (64-bit LE and beyond): heap cells are read and written
  with explicit little-endian `from_le_bytes`/`to_le_bytes` (heap/layout.rs; the only
  `_ne_bytes` is an ASCII test that ORs words, endian-neutral), offsets are `u32`, and
  wasm32 builds already embed a base made by a 64-bit build machine. Floats computed while
  making the base go through djs-libm, whose arch routines (sqrt, fma) are correctly
  rounded, so x86 and aarch64 agree.
- Git dependency: files outside the crate directories (builtins/gen/prelude.js via
  `include_str!`, djs-core's build hash over ../djs-libm/src and builtins/gen/*.js,
  djs-prelude's registry.tsv when `registry` is on) resolve in cargo's git checkout,
  which is the whole repository. djs-embed turns `registry` off anyway. A crates.io
  package would need them inside the crate (not now: publish = false).
- A consumer's dev profile builds djs-core at opt-level 0: the example's run takes 0.35 s
  vs 0.045 s at the workspace's opt-level 1. The docs recommend the profile override.

- Docs: djs-embed's crate docs (src/lib.rs "Depending on djs-embed"), guide
  docs/guide/rust-api.md ("Adding it to a project", the example in "Where to look"),
  TESTING.md "Cross builds of djs-embed", D3220 (+ index line). No djs-prelude change needed.

## Follow-ups
- Publishing to crates.io would need the files djs-core reads outside its directory
  (builtins/gen/prelude*.js, ../djs-libm/src for the build hash) inside the packaged crates;
  git and path dependencies do not.
- Big-endian targets: the image encoding is explicit little-endian, so they should work, but
  none is built or tested.

## Result
`scripts/cross-check.sh` (manual, not a gate leg; TESTING.md "Cross builds of djs-embed")
builds `crates/djs-embed/examples/cross_check.rs` for aarch64/x86_64-apple-darwin and
-unknown-linux-musl (`zig cc` linker wrappers in `scripts/cross/`), runs them natively, under
Rosetta and in docker, and requires identical output on all four, including that the embedded
base image equals the one each target makes at run time. `--release`, `--consumer REV` (a
scratch project depending by `git = "file://..."` + `rev`, fat-LTO release profile).
Run once here: all four targets build and run with identical output in dev, release and
consumer-by-git-rev (dev and fat-LTO release) builds; base 188,672 bytes, identical on every
target (D3220 says why). Runtime crates on every target: djs-embed, djs-core, djs-prelude,
djs-libm. Release fat-LTO stripped binary of the example: 2.4-2.5 MB per target (static musl).
How hosts depend on it: djs-embed's crate docs and docs/guide/rust-api.md (path or git rev,
the `unicode` feature, `[profile.dev.package.djs-core] opt-level = 1`, panics must unwind).
