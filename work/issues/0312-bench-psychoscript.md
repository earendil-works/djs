# 0312: Psychoscript in the engine benchmarks

status: done
agent: ppsycho
branch: issue/0312-bench-psychoscript
depends-on: -
touches: js/bench/engines/ (engines.ts, run.ts, bridge.ts, ops.ts if it fits, package.json, package-lock.json, prelude.js only if a workload needs it, driver.c for the QuickJS releases' limits, a new traps.ts), scripts/bench-engines.sh, scripts/status.py, tests/scores/status.tsv, crates/djs-docs (status.rs gen:status tables/charts, only as needed for a new engine column), docs/guide/benchmarks.md, docs/decisions.md
guide: docs/guide/benchmarks.md

## Goal (owner)
Add Psychoscript (https://psychoscript.com/, https://github.com/anomalyco/psychoscript,
npm `psychoscript`, latest 0.4.0, MIT, by anomaly) to the benchmarks page next to the
QuickJS builds and monty.

What it is: "a confined, resumable JavaScript interpreter": a pure-JavaScript package
(no wasm; depends on acorn) for Node >= 24 / Bun >= 1.4 that compiles a strict-mode subset
of JavaScript to bytecode and runs it on a VM written in JS, with host functions
(`Host.fn`, sync or async), `Runtime.make({ globals, limits })`, `runtime.execute(code)`,
suspend to bytes and resume. Codemode bodies (top-level await and return) are its target,
like ours. Not supported, among others: eval/new Function, Proxy/Reflect,
Object.setPrototypeOf, private members, typed arrays other than Uint8Array, Symbol(),
timers, TypeScript syntax, Intl.

## Acceptance criteria
- [x] `psychoscript` pinned in js/bench/engines/package.json + package-lock.json (integrity
      hashes), installed by scripts/bench-engines.sh like the other npm engines.
- [x] A `psychoscript()` engine in engines.ts running every codemode workload with a fresh
      runtime per run, through the same tool bridge (one JSON string of tool answers, the
      prelude's JSON work inside the engine), and returning node's result string. If a
      workload or the prelude uses something it lacks, say exactly what in the log; prefer
      an equivalent rewrite of the prelude that keeps every engine's work the same over
      dropping the workload, and never change what other engines measure without saying
      so. `ts`: types stripped by the host, as for the QuickJS engines.
- [x] The bridge benchmark (bridge.ts) covers it through its best host path (Host.fn
      arguments/results copied across), if the API allows the cases; otherwise say why.
- [x] Recorded numbers for it in tests/scores/status.tsv (scripts/status.py engines /
      bridge), so the generated tables and charts of benchmarks.md get a Psychoscript
      column. Don't re-record the other engines' rows under load > ~4 (see the board's
      "next quiet re-record"); if status.py can only record all engines at once, add a way
      to record one engine's keys, or record when the machine is quiet, and say which.
- [x] benchmarks.md explains it honestly: it is not wasm, so it runs in the host's own JS
      engine (V8's JIT, the host's GC and heap) rather than in a sandboxed linear memory.
      The columns that don't mean the same (download size: the JS package, gzip/brotli of
      what a host ships, no wasm; "compile" = import/load; memory: no linear memory, so
      either a defensible measure clearly labelled (e.g. V8 heap delta, with its caveats)
      or "n/a") must say so in the table notes and the prose. Its isolation model differs
      (same heap as the host) and belongs in one sentence, without judging.
- [x] `cargo run -q -p djs-docs -- check` clean; the gate's codemode leg still runs only djs.

### Scope addition (owner): runaway scripts
- [x] A separate script (js/bench/engines/traps.ts, not part of the timed run.ts) runs about
      eight hostile/runaway cases on every engine of the page: a sync endless loop
      (`for(;;){}`); an endless async/microtask loop (a promise chain that never settles the
      run); growing an array/object until memory runs out; one huge allocation
      (`"x".repeat(2**30)` or `new Array(2**30).fill(0)`); a string that doubles until it
      fails; deep recursion (stack overflow); deep nesting into JSON.parse/JSON.stringify;
      catastrophic regexp backtracking (`/(a+)+$/` on a long non-matching string). Others only
      if they show a real difference.
- [x] Every engine with its best documented limit mechanism, configured the same where
      possible (1 s time budget, 64 MiB memory cap): djs's fuel/memory limits
      (docs/guide/limits.md); QuickJS interrupt handler + JS_SetMemoryLimit + max stack size
      in driver.c for the two releases; quickjs-emscripten's setInterruptHandler /
      setMemoryLimit / setMaxStackSize; quickjs-wasi's equivalents; monty's limits;
      Psychoscript's timeoutMs/maxOutputBytes and fixed caps (no memory limit; a regexp
      match runs on the host engine, not interrupted). No mechanism: say so, don't invent one.
- [x] Per engine x case: what happened (stopped by a limit / catchable error in the script /
      uncatchable termination / host process crash or hang / ran until the outer kill), time
      until it stopped, memory high-water (linear memory; Psychoscript: the host process's
      RSS/heap, labelled), and whether the same host process can create and run a fresh VM
      afterwards.
- [x] Each engine x case in its own child process with a hard outer timeout (~10 s) and a
      memory guard, so a crash or OOM doesn't take the harness down.
- [x] benchmarks.md gets a "Runaway scripts" section: one table of outcomes (short cells,
      details in notes), generated from recorded results if practical, else a dated table
      with the command to re-run it. Factual, djs's weaknesses included; links limits.md.

## Notes
- Precedent: monty (0229/0238) is a non-QuickJS, non-JS-in-wasm engine already on the
  page; follow how it was integrated (engines.ts `monty()`, run.ts, status.py keys
  `engines/<slug>/...`, djs-docs status.rs columns).
- Node here is v26; Psychoscript needs >= 24.
- Its host-call model (async `Host.fn` returning promises, `Promise.all`) should map to the
  prelude's tool calls; check how the other engines do the tool call (sync from the
  recorded answers) and do the same, sync if it allows.
- The page's "In short" summary and engine list must stay true with an eighth engine.
- Benchmark with `scripts/timeout.sh`, `nice`, and say the load in the log.

## Log
- Load during the work: 3.5 to 22 (other agents' gates); recorded at 3.5 (engines), 4.2
  (bridge), 4.0 (traps). Other engines' engine/bridge rows were not re-recorded.
- Psychoscript 0.4.0 pinned (with acorn 8.16.0; its optional peer `effect` is not
  installed). It runs all nine workloads and the prelude unchanged: codemode bodies as
  given, `__DATA` as a `bindings` value, a fresh `Runtime.make({})` per run, the result
  string as the run's value; `ts` with the host's stripped types. Nothing in the prelude
  or workloads needed a rewrite; no other engine's measurement changed by it.
- Size: no wasm (`files` []), `glue` = dist/**/*.{js,json} without dist/effect plus
  acorn/dist/acorn.mjs: exactly what its import loads (checked with module.registerHooks),
  1,199 KB raw, 303 KB gzip, 254 KB brotli. Compile = the import (18 ms). New VM =
  Runtime.make (~0); its realm is built inside each execute. Memory: n/a (Vm.memory()
  returns null; status.py skips None).
- Bridge: no persistent VM, so put = a run of `get(); return 0` (Host.fn returning the
  value) and get = a run of `return get()`; `BridgeVm.overhead` gives the run without
  the operation (put: `none(); return 0`; get: the put run), bridge.ts times each run
  alone, alternating, and subtracts the medians (floored at 0). Strings are the host's
  own strings (no copy: ~0); bytes cannot cross out (`InvalidDataValue`), they come out
  via `toBase64()` and are decoded on the host; nested 100 fails at its 32-level cap.
  bridge.ts awaits put/get/call only when they return a promise (no microtask for the
  others).
- status.py: `engines --engines a,b` and `bridge --engines a,b` record only those
  engines' keys, their load as engines/<e>/load and engines/bridge/<e>/load (no ops, no
  payload sizes); djs-docs prints such a load beside the table's ("load 3.1
  (Psychoscript: 3.5)").
- djs-docs: ENGINES has 8 entries; `is_wasm` (Psychoscript not): size row "none" + JS in
  the glue column with raw/brotli in the note, startup idle "n/a", memory tables and
  charts "n/a"/no bar, bridge strings "no copy", geometric means over the wasm engines'
  rows (n/a for Psychoscript, so the others' means are unchanged).
- Headline (Psychoscript vs djs): run time over the set 1.62x djs's (quickjs-wasi 1.60x,
  QuickJS 1.38x); second to djs on fanout/media/loop, behind the QuickJS releases on
  json/ts/heavy (heavy 20.1 vs 8.9 ms). Bridge: JSON 1 MB in 4.7 vs 3.1 ms, out 1.22 vs
  1.24 ms; 10 MB out 33 vs 13 ms; strings no copy; e2e: 1,000 calls 1.7 vs 3.2 ms, edit
  1 MB 0.17 vs 0.37 ms, field of 1 MB 5.6 vs 3.3 ms.
- Runaway scripts (owner's scope addition): js/bench/engines/traps.ts, `Engine.limited`
  and `trapProgram` per engine in engines.ts, nine cases (loop, promise jobs, growing
  array, 1 GiB string, doubling string, recursion, JSON.parse 1e6 deep, JSON.stringify
  1e5 deep, /(a+)+$/), each in a child process (10 s kill, 4 GiB RSS guard), limits 1 s
  and 64 MiB. Findings: every engine stops loops and promise jobs at 1 s; djs catches
  OOM, length, stack; Bellard's QuickJS throws `null` on OOM and its handler has no room
  for a message; the emscripten builds hit OOM at 358 MiB under a 64 MiB limit; QuickJS-NG
  0.18's `string too long` from concatenation is uncatchable; monty's memory limit ends
  runs uncatchably and its regex finishes the backtracking case at once; Psychoscript has
  no memory limit (1.4 GB RSS in 1 s) and its regex match blocks the host until killed.
  Every engine that ended its run ran a fresh VM afterwards.
- Found while wiring QuickJS limits: (1) our Bellard WASI build had CONFIG_STACK_CHECK off
  (the sed for CONFIG_ATOMICS matched the next identical `#if` too), so deep recursion
  ran off the 64 KiB stack into a wasm trap; (2) it returned 0 from
  js_def_malloc_usable_size although wasi-libc has malloc_usable_size, so the memory
  limit and the GC's allocation count only saw per-block overhead (a 64 MiB limit let
  the heap reach 350 MiB). Fixed in bench-engines.sh (perl for the Atomics `#if` only,
  the Linux branch for wasi) and driver.c (JS_SetMaxStackSize 48 KiB in bench_new: with
  the check on, the 1 MiB default is wrong for a 64 KiB stack and every call failed).
  This changes the QuickJS release build the page measures: run times measured within
  noise of the recorded ones (heavy 15.5 vs 15.1 ms, memory identical), module size
  +2 KB; the recorded QuickJS rows were not re-recorded (load). The stamp now hashes
  bench-engines.sh too, so build changes rebuild.
- The npm QuickJS packages with their default 1 MiB maxStackSize overflow the host's
  native stack first on deep recursion/JSON (node's RangeError out of the module, a trap);
  all QuickJS builds get 48 KiB in the traps run.
- Guide: docs/guide/benchmarks.md (In short, what is measured, engines, sizes, startup,
  run time, memory, bridge, a new "Runaway scripts" section, re-running, where to look);
  status.md only regenerated. Decisions D3762, D3763.

## Follow-ups
- Re-record the QuickJS (Bellard) engine rows when quiet (`status.py engines --engines
  quickjs`, or with the next full quiet re-record): its build now has the stack check and
  real allocation sizes; its recorded size is ~2 KB off.
- Psychoscript's microtask scheduler: `const spin = () => Promise.resolve().then(spin)`
  (a self-resolving chain) ends at once with "waiting on a promise that can never settle"
  although jobs are queued; the flat variant used in traps.ts times out as expected.
  Possibly an upstream bug, not on the page.
- traps.ts is not a gate leg (child processes, 10 s kills); a `--quick` smoke (djs only,
  short budget) could be one if wanted.

## Result
Psychoscript is the benchmarks page's eighth engine (run time, sizes, startup, bridge;
memory n/a, honestly labelled), and the page has a "Runaway scripts" section with two
generated tables (outcomes, memory) for all eight engines under each one's own limits.
Run: `scripts/bench-engines.sh --engines psychoscript`, `node js/bench/engines/traps.ts`;
record: `scripts/status.py engines --engines psychoscript`, `scripts/status.py traps`.
Known gaps: the other engines' rows are from 2026-11-21 (djs at d0bc2eef16), the QuickJS
release build changed since (see Follow-ups); Psychoscript's bridge in/out are
differences of whole runs.
