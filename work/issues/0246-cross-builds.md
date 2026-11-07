# 0246: djs-embed for Gondolin's targets: cross-builds, image portability, consuming it

status: open
agent: -
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
- [ ] cross-build script; all four targets build; binaries run in docker / Rosetta with the same
      output; base image identical across targets (or the reason it isn't, and a fix)
- [ ] docs on depending on djs-embed by git rev; a scratch consumer builds by `git = file://`
- [ ] runtime crate list recorded

## Log

## Follow-ups

## Result
