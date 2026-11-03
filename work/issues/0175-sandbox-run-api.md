# 0175: Sandbox run API and the wasm dumper packaging

status: done
agent: example
branch: issue/0175-sandbox-run-api
depends-on: 0172
touches: js/src/sandbox.ts, js/test/, crates/djs-wasm, scripts/build-wasm.sh, js/proto (remove dump.ts, D2564), js/package.json
guide: -

## Goal
Hosts can stop at an idle point, dump and resume (docs/continuation-dumps.md "Sandbox API").

## Acceptance criteria
- [x] `sandbox.begin(code, params)` returns a run with `state`, `calls`, `settle(id, outcome)`,
      `advance()` and `dump()` (idle only); `Sandbox.resume(text, opts)` returns the sandbox and
      the run with its open calls. `runAsync` is rebuilt on top, its behavior unchanged
      (journals included).
- [x] wasm exports for loading and dumping format 2; both packagings build: the main module
      with the `dump` feature, and `djs-dump.wasm` next to a dumper-less `djs.wasm` with the
      image transport (persist: image -> dump module -> dump; resume: dump module -> image ->
      main module). Both are exercised by the tests; sizes recorded.
- [x] js/test/dumpresume.test.ts: round trip at every idle point for scripts with parallel
      tools, loops, generators, classes, rejections; refusals as SandboxErrors.
- [x] js/proto/dump.ts and its comparison test removed (D2564).

## Log

- The first piece landed with 0179 (D2630–D2632): `begin`/`Run`/`Sandbox.resume`,
  `vm_load_dump` returning Loaded, `vm_dump` with the host's name table and handles.
- `run`/`runAsync`/`replay` rebuilt on the host-driven loop (D2634): `drive` runs to the
  next idle point and lists new async calls; `auto` adds, through two hooks, calling the
  async functions at once and passing results back before every slice, as before, and
  waiting/replaying at idle points. All journal and sandbox tests pass unchanged.
  `begin(..., { journal })` records host-driven runs; replay runs them.
- Holding the run's result handle across idle points changed the canonical hash (the
  handle table), so a journaled host-driven run did not replay. Now `vm_dump`'s handle 0
  names the entry call's outcome and no handle is kept; `vm_outcome` removed (D2633). A
  dump leaves the canonical hash unchanged (tested).
- js/test/dumpresume.test.ts: dump + resume in a fresh sandbox at every idle point for
  parallel tools, nested loops, sync/async generators and for-await, classes (methods,
  statics, extends, super), rejections (try/catch, allSettled, a pending .catch);
  transcripts equal the uninterrupted run's. Refusals: a WeakMap, a finished run, an
  unknown tool, not a dump, an unknown site. Class instance fields are refused by the
  exporter (reported to portable): a `todo` test.
- Packaging B (D2635): `SandboxOptions.dumpModule`; dump/load/resume move the arena to a
  transient sandbox of the dump module and back. `scripts/build-wasm.sh --dump` builds
  target/djs-dump.wasm; the gate also builds a debug module without `dump`
  (target/wasm-nodump, `DJS_WASM_NODUMP`; scripts/gate.sh touched for that, minimal), and
  dumpresume runs every case in A, B with one module, and B with the two real modules.
  Sizes: 1,038,040 B / 443.8 KB gzip without the dumper, 1,312,145 B / 540.6 KB gzip with
  it; B adds 0–1 ms per dump or resume.
- js/proto was already removed in 0170 (D2564). docs/functions-as-code.md still mentions
  js/proto/dump.ts and its comparison (not in this issue's touches): follow-up.

## Follow-ups
- portable: the exporter refuses instance fields of a class declared in a waiting function
  (`C~<fields 0>: an internal function of a class`); js/test/dumpresume.test.ts has it as
  a `todo`. Also `r: undefined` instead of `$dump.TDZ` for a pending `const r = await …`.
- docs/functions-as-code.md: drop the js/proto/dump.ts comparison text (removed in 0170).
- docs/continuation-dumps.md "One dumper, packaging open": the measured sizes (D2635).

## Result

- Sandbox run API: `begin(code, params, { filename, journal })` -> `Run { state, calls,
  value, error, settle, advance, dump }`; `Sandbox.resume(text, opts)`; `load()` returns
  `{ calls }`. `run`/`runAsync`/`replay` run on the same loop, behavior unchanged.
- Dumps leave the VM untouched (handle 0 = the entry call's outcome); recorded runs can be
  dumped and still replay.
- Packagings: A (module with `dump`) and B (`dumpModule`: image transport), both in the
  gate's js tests; `scripts/build-wasm.sh --dump`; sizes and timings in docs/host.md.
- Decisions D2633–D2635.
