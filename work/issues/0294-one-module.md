# 0294: One wasm module with the dumper, one package entry point, npm 0.2.0

status: in-progress
agent: pone
branch: issue/0294-one-module
depends-on: -
touches: scripts/build-wasm.sh, scripts/npm-pack.sh, scripts/npm-test.sh, scripts/gate.py, scripts/size/, tests/scores/size.tsv, crates/djs-wasm/ (Cargo features, vm_replace_image), js/src/ (index, dump, slim, sandbox, abi), js/test/, js/pkg-test/, js/package.json, js/README.md, docs/decisions.md, docs/guide
guide: docs/guide/js-api.md, docs/guide/dumps.md, docs/guide/host.md, docs/guide/pi.md (and pages quoting module sizes)

## Goal
Owner, 2026-11-23: djs ships **one** wasm module, and it includes the dumper. p2 and pi
use the same distribution: pi's code mode (coding agent, rollups) and pi's durable code
mode (which dumps before every launched call and when it parks) load the same file. The
size grows; bringing it down again is later work.

Today there are three release modules (main without the dumper, slim, dump) and the
Sandbox has packaging B (D2635, D2701, D2704): `SandboxOptions.dumpModule` copies the VM's
heap image into an instance of the dump module whenever a dump is asked for. With one
module that has the dumper, that machinery has no purpose. The npm package
`@earendil-works/djs` (0.1.0 on npm, 605 commits old) has five entry points, three of
them the same API with another default module (D3106).

After this issue:
- `scripts/build-wasm.sh` (no flag) builds the one module: the `dump` feature (with what
  it pulls in today: lazy, compile-cache, the prelude names) plus the shipped build's
  `unsafe-heap` and `cold-unicode`. Its file is the package's `wasm/djs.wasm`. Check that
  `dump` + `unsafe-heap` builds and passes (the dump module so far kept the checks).
- The Sandbox has no `dumpModule`: `viaDumper`, the cached dump-module sandboxes,
  `takeImage`/the image transport and `vm_replace_image` go (unless something else needs
  the export: check). A module without the dumper (a debug or custom build) throws the
  existing clear error on `dump`.
- The package has one entry point `.` (the whole API, `defaultWasmPath()` = djs.wasm, plus
  dump-runtime's exports `helpers`, `script`, `scriptFor`), `./wasm/*.wasm` and
  `./package.json`. `./dump`, `./slim`, `./dump-runtime` go; js/src/dump.ts and slim.ts go.
  The package ships only djs.wasm.
- The slim build leaves the package. Its build (`--slim`) and gate legs stay for now (a
  size reference and pi's adapter tests); say in the log what still uses it.
- The size gate (scripts/size/budget.py, tests/scores/size.tsv) checks the new module with
  a new ceiling: record it as an owner decision ("one module with the dumper, the size
  grows, owner 2026-11-23"), with the measured gzip. Re-record what status.py measures
  for the module (size, startup, memory rows) if they are gate inputs; otherwise list
  them as stale.
- js/package.json version 0.2.0. js/README.md and the guide describe one module.
- Decision entries: one that supersedes D2635/D2701/D2704 (packaging B) and D3106
  (entry points per module), with the reason (one distribution for every host).

## Acceptance criteria
- [ ] one release module with the dumper, built by `scripts/build-wasm.sh`; `--dump` gone or an alias (say which)
- [ ] no `dumpModule` / image transport in js/src; tests that covered both packagings test the one module
- [ ] package: entry points `.`, `./wasm/*.wasm`, `./package.json`; only djs.wasm; `scripts/npm-test.sh` passes (bundler tests copy the one module)
- [ ] size gate on the new module with the owner's new ceiling
- [ ] guide pages, js/README.md, decisions updated; `djs-docs check` clean
- [ ] version 0.2.0 (the owner tags and publishes; do not push or tag)
- [ ] a packed build for pi at `/Users/mitsuhiko/Development/djs/work/scratch/pi-pack/` (see Notes)

## Notes
- pi consumes djs through `@earendil-works/djs` as a `file:` dependency on
  `/Users/mitsuhiko/Development/djs/work/scratch/pi-pack/package` while this is unreleased
  (`scripts/npm-pack.sh --out /Users/mitsuhiko/Development/djs/work/scratch/pi-pack`). The
  coordinator packed main's current layout there for pi to start with. Once your branch
  has the new layout, pack it there again and tell the coordinator, so pi can move to
  `wasm/djs.wasm`.
- js/pi/ (the adapter that runs pi's QuickJS prelude on djs) is on its way out: pi moves
  to host-driven runs and drops QuickJS. Keep it working here; retiring it is a follow-up.

## Log

## Follow-ups

## Result
