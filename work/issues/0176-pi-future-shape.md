# 0176: pi's future shape: prelude-next, adapter mode, dumps

status: done
agent: example
branch: issue/0176-pi-future-shape
depends-on: 0175
touches: tests/pi/prelude-next.ts (new), js/pi/, js/test/pi-vm.test.ts, docs/host.md
guide: -

## Goal
Build the shape pi should move to (D2565) in djs's own adapter, and persist pi sessions as
continuation dumps. pi itself is not changed.

## Acceptance criteria
- [x] tests/pi/prelude-next.ts: pi's prelude with tools as engine async host functions, the
      body's promise as a host handle, `finished` and the store's writes in the adapter; the
      differences listed at its top. tests/pi/prelude-source.ts stays verbatim.
- [x] An adapter mode that uses it, with `dumpSession()` / `resumeFromDump()`; the prelude is
      rerun on resume (deterministic, D2262) and the dump names its globals as host globals.
- [x] pi's suite (`node js/pi/pi-tests.ts`) passes in both modes; in the new mode also with
      `--dump-every-idle`, each wait resumed in a new module instance.
- [x] pi-vm.test.ts covers dump and resume through the adapter, with the open-call policies
      (deliver recorded, re-issue, reject).
- [x] docs/host.md describes the new shape as what pi should adopt.

## Log

- tests/pi/prelude-next.ts (D2650, D2652): pi's prelude with tools as calls of an async
  host function `call(kind, name, argsJson)`, the body's promise/`finished`/output
  counters/store in the adapter, `decode`/`failure`/`encode`/`describe` helpers, and no
  lockdown (frozen builtins instead: the lockdown's accessors on builtins cannot be
  dumped; pi's suite without lockdown on frozen builtins fails the same 11 as baseline).
- js/pi/vm.ts `shape: "next"`: calls numbered for pi by the adapter, results decoded in
  the VM, `done` reported once the queue ran empty, stall detection by status. Calling
  prelude functions while async calls are pending needs their return value at IDLE:
  `vm_outcome` is back (removed in 0175 as unused). `dumpSession()` -> `{ dump, state }`,
  `resumeFromDump()`, `openCalls()`, `dumpEveryIdle` (D2651).
- Found while running pi's suite in-thread: a loop of sync host calls never polls
  `shouldInterrupt` (each `vm_run` returned at the host call with fuel left, and the next
  got full fuel), so `while (true) tools.echo(1)` hung pi's mode too. `vm_fuel_used` +
  fuel carried across host calls (D2653), DETERMINISTIC_VERSION 3.
- pi's suite (release module): pi mode 215/226, `--next` 215/226, `--next
  --dump-every-idle` 215/226 (djs-dump.wasm), the same 11 failures each.
- pi-vm.test.ts runs every test in pi, next and next+dumpEveryIdle (the lockdown test is
  skipped in next), plus a dump/resume test with the three open-call policies and a todo
  for Math.random's state (portable, 0182).
- Gate (coordinator's call): every merge runs pi's suite with `--next` on target/size
  (tests/pi/expected-failures.txt: the 11 baseline failures); `--next --dump-every-idle`
  is scripts/pi-dump-every-idle.sh (builds target/djs-dump.wasm), for the gate's full run.
- Docs: docs/host.md "The shape pi should adopt", the fingerprint row, the new exports;
  continuation-dumps.md's measured packaging sizes; functions-as-code.md without the
  js/proto comparison.

## Follow-ups
- portable (0182): dumps carry `Math.random`'s state; then drop the `todo` of "next: a
  dump keeps Math.random's sequence" in js/test/pi-vm.test.ts.
- Packaging B in the pi adapter (a dump module beside the release module) would make the
  dump-every-idle suite run as cheap as `--next` (no extra LTO build).
- The Sandbox's `maxFuel` counts only slices that ran out (`used += slice`), so sync host
  calls in a loop do not use up its budget either; `vm_fuel_used` (D2653) would fix it.

## Result

- tests/pi/prelude-next.ts: pi's prelude in the shape pi should adopt (D2565, D2650,
  D2652), differences listed at its top; prelude-source.ts untouched.
- js/pi/vm.ts `shape: "next"`: `dumpSession()` -> `{ dump, state }` (JSON),
  `resumeFromDump(saved, { wasm, prelude, data, ... })`, `openCalls()`, `dumpEveryIdle`
  (D2651). pi's mode unchanged except the fuel fix and a frozen `performance`.
- Fixed in both modes: loops of sync host calls are interrupted (`vm_fuel_used`, D2653;
  DETERMINISTIC_VERSION 3). `vm_outcome` back for calls between results.
- pi's suite: 215/226 in pi's mode, `--next`, and `--next --dump-every-idle` (the same 11
  failures, tests/pi/expected-failures.txt). Gate: `--next` every merge;
  scripts/pi-dump-every-idle.sh for the full run.
- Known gap: Math.random's state is not in dumps yet (0182; a todo test).
