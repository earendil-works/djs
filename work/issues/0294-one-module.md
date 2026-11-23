# 0294: One wasm module with the dumper, one package entry point, npm 0.2.0

status: done
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
- [x] one release module with the dumper, built by `scripts/build-wasm.sh`; `--dump` gone or an alias (say which): gone
- [x] no `dumpModule` / image transport in js/src; tests that covered both packagings test the one module
- [x] package: entry points `.`, `./wasm/*.wasm`, `./package.json`; only djs.wasm; `scripts/npm-test.sh` passes (node, bun, js-tests; browser not run here: needs the network for playwright)
- [x] size gate on the new module with the owner's new ceiling
- [x] guide pages, js/README.md, decisions updated; `djs-docs check` clean
- [x] version 0.2.0 (the owner tags and publishes; do not push or tag)
- [x] a packed build for pi at `/Users/mitsuhiko/Development/djs/work/scratch/pi-pack/` (see Notes)

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

2026-11-23 (pone)
- Measured first (wasm profile, gzip -9): main's shipped module 999,705 raw / 432,882 gzip;
  `dump` + `unsafe-heap` + `cold-unicode` builds and runs: 1,875,103 / 767,573 (+77%);
  main's old checked dump module 1,893,434 / 778,007. The dumper is ~720 KB raw code
  (vm::dump: plain::write, Dumper::visit_activation/finish/emit_*, cont::G), compiler +93 KB
  raw (smap, compile_cache). Told the coordinator; the owner accepted the growth.
- scripts/build-wasm.sh: default = `unsafe-heap,cold-unicode,dump`; `--dump` removed (callers
  updated: npm-pack, bench-engines, pi-dump-every-idle, sleep-dump's run.sh, status.py).
- js/src/sandbox.ts: `dumpModule`, the `dumpers` cache, `RawImage`, `viaDumper`, `takeImage`,
  `replaceVm`, the image constructor path gone; `vm` is readonly. The no-dumper error now says
  the module "was built without the `dump` feature".
- djs-wasm: `vm_replace_image` and `Vm::replace_image` removed (no other user); ops renumbered,
  ABI_VERSION 7 -> 8 (js/src, js/pi). `vm_from_bytes` stays (pi's prelude images).
- Package: js/src/index.ts is the one entry point (also `helpers`, `script`, `scriptFor`);
  dump.ts and slim.ts deleted; package.json 0.2.0 with `.`, `./wasm/*.wasm`, `./package.json`;
  npm-pack `--module FILE`; smoke.js dumps and resumes with the default module.
- npm-test js-tests had been broken on main since 0291/0292 (golden-host reads
  tests/dump/v2/golden-host; imports.test.ts and nesting.test.ts import internal modules):
  now copies golden-host's files and leaves those two out. node, bun and js-tests pass on the
  packed tarball; deno is not installed here; browser not run.
- js tests: the packagings loops collapse to the one module (durable, imports, task,
  dumpresume); golden-host still loads in the no-dumper build (loading needs no dumper); a new
  test checks the clear error of a module without the dumper. All 154 pass on the dev modules.
- Gate: build:size and the release clippy "main" module get `dump` (+cold-unicode); the
  clippy "dump" module leg goes (main covers it); codemode workloads on the dev module alone;
  npm package leg packs `--module` the size build; the full-only pi dump-every-idle leg runs on
  the size build (`scripts/pi-dump-every-idle.sh MODULE`). build:wasm-nodump stays: js tests'
  no-dumper cases (debugger option, callSite, plain-dump load, golden-host load, the dump
  error).
- What still uses the slim build: the gate's build:size-slim -> size budget `--slim`, pi tests
  (slim without lockdown); build:slim-tests/slim-bins (native slim) and the slim clippy legs;
  status.py size (slim/*); docs (language.md, builtins.md, host.md module table).
- Size gate: ceiling_gzip 432,899 -> 768,597 (767,573 + slack), D3601. codemode-memory.tsv
  re-recorded (+192 KiB every row: initial memory 2 -> 4 pages). Checked on the release
  module: nesting test, codemode memory, pi tests next mode (221/226, 0 new) and with a dump at
  every idle point (0 new failures), release clippy of the new feature set.
- status.py: `size` measures the module, slim and a no-dumper build (`dump/extra-gzip` =
  334,869); dump/bytes and dump/gzip rows dropped; `pi`, `dumps`, `bounds` on the one module
  (bounds' checked build now has `dump,cold-unicode` too). Re-recorded `size` only.
- Decisions D3600 (one module, packaging B goes; amends D3100: the dumper runs unchecked),
  D3601 (owner's size ceiling), D3602 (one entry point, 0.2.0).
- Guide pages updated: js-api.md, dumps.md, host.md, pi.md, continuations.md, arena.md,
  determinism.md, goals.md, language.md, snapshots.md, testing.md; generated sizes
  regenerated (status.md, README.md, index.md). Also ARCH.md, TESTING.md, docs/host.md,
  docs/continuation-dumps.md, docs/functions-as-code.md, js/README.md, publish.yml.
- Packed for pi at work/scratch/pi-pack (0.2.0) and told the coordinator.

## Follow-ups
- Bring the module's size down again (the dumper is ~335 KB gzip): the owner's later work.
- Stale status.tsv rows measured on the old module (not gate inputs): startup/*, heap/*,
  bounds/*, pi bench (pi/*-ms), dumps/*, engines/* (benchmarks page: djs's size, startup,
  memory), bridge/*. Re-record on a quiet machine (`scripts/status.py startup heap bounds
  dumps engines`).
- npm-test.sh deno and browser were not run here (no deno; browser needs the network).
- js/pi (the QuickJS-prelude adapter) is on its way out; retire it and then the slim gate legs
  that only serve it.
- scripts/__pycache__/*.pyc are tracked in git on main (stale bytecode of gate.py etc.):
  untrack and ignore them.

## Result
djs ships one wasm module, with the dumper: `scripts/build-wasm.sh` (no flag; `--dump` is
gone) builds target/wasm32-unknown-unknown/wasm/djs_wasm.wasm with `unsafe-heap,cold-unicode,
dump`: 1,875,103 B raw, 767,573 B gzip (before: 999,705 / 432,882 without the dumper). The
Sandbox has no `dumpModule`; dumping runs in the sandbox's own VM, and a module without the
dumper throws a clear `misuse` error. `vm_replace_image` is gone (ABI 8). The npm package
@earendil-works/djs 0.2.0 has one entry point `.` (the API plus `helpers`, `script`,
`scriptFor`), `./wasm/*.wasm`, `./package.json`, and ships only wasm/djs.wasm
(`scripts/npm-pack.sh [--module FILE] [--out DIR]`). The size gate's ceiling is 768,597 B gzip
(D3601). Known gaps: see Follow-ups (stale status rows, deno/browser package tests not run
here).
