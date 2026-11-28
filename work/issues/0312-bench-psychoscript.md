# 0312: Psychoscript in the engine benchmarks

status: in-progress
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
- [ ] `psychoscript` pinned in js/bench/engines/package.json + package-lock.json (integrity
      hashes), installed by scripts/bench-engines.sh like the other npm engines.
- [ ] A `psychoscript()` engine in engines.ts running every codemode workload with a fresh
      runtime per run, through the same tool bridge (one JSON string of tool answers, the
      prelude's JSON work inside the engine), and returning node's result string. If a
      workload or the prelude uses something it lacks, say exactly what in the log; prefer
      an equivalent rewrite of the prelude that keeps every engine's work the same over
      dropping the workload, and never change what other engines measure without saying
      so. `ts`: types stripped by the host, as for the QuickJS engines.
- [ ] The bridge benchmark (bridge.ts) covers it through its best host path (Host.fn
      arguments/results copied across), if the API allows the cases; otherwise say why.
- [ ] Recorded numbers for it in tests/scores/status.tsv (scripts/status.py engines /
      bridge), so the generated tables and charts of benchmarks.md get a Psychoscript
      column. Don't re-record the other engines' rows under load > ~4 (see the board's
      "next quiet re-record"); if status.py can only record all engines at once, add a way
      to record one engine's keys, or record when the machine is quiet, and say which.
- [ ] benchmarks.md explains it honestly: it is not wasm, so it runs in the host's own JS
      engine (V8's JIT, the host's GC and heap) rather than in a sandboxed linear memory.
      The columns that don't mean the same (download size: the JS package, gzip/brotli of
      what a host ships, no wasm; "compile" = import/load; memory: no linear memory, so
      either a defensible measure clearly labelled (e.g. V8 heap delta, with its caveats)
      or "n/a") must say so in the table notes and the prose. Its isolation model differs
      (same heap as the host) and belongs in one sentence, without judging.
- [ ] `cargo run -q -p djs-docs -- check` clean; the gate's codemode leg still runs only djs.

### Scope addition (owner): runaway scripts
- [ ] A separate script (js/bench/engines/traps.ts, not part of the timed run.ts) runs about
      eight hostile/runaway cases on every engine of the page: a sync endless loop
      (`for(;;){}`); an endless async/microtask loop (a promise chain that never settles the
      run); growing an array/object until memory runs out; one huge allocation
      (`"x".repeat(2**30)` or `new Array(2**30).fill(0)`); a string that doubles until it
      fails; deep recursion (stack overflow); deep nesting into JSON.parse/JSON.stringify;
      catastrophic regexp backtracking (`/(a+)+$/` on a long non-matching string). Others only
      if they show a real difference.
- [ ] Every engine with its best documented limit mechanism, configured the same where
      possible (1 s time budget, 64 MiB memory cap): djs's fuel/memory limits
      (docs/guide/limits.md); QuickJS interrupt handler + JS_SetMemoryLimit + max stack size
      in driver.c for the two releases; quickjs-emscripten's setInterruptHandler /
      setMemoryLimit / setMaxStackSize; quickjs-wasi's equivalents; monty's limits;
      Psychoscript's timeoutMs/maxOutputBytes and fixed caps (no memory limit; a regexp
      match runs on the host engine, not interrupted). No mechanism: say so, don't invent one.
- [ ] Per engine x case: what happened (stopped by a limit / catchable error in the script /
      uncatchable termination / host process crash or hang / ran until the outer kill), time
      until it stopped, memory high-water (linear memory; Psychoscript: the host process's
      RSS/heap, labelled), and whether the same host process can create and run a fresh VM
      afterwards.
- [ ] Each engine x case in its own child process with a hard outer timeout (~10 s) and a
      memory guard, so a crash or OOM doesn't take the harness down.
- [ ] benchmarks.md gets a "Runaway scripts" section: one table of outcomes (short cells,
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

## Follow-ups

## Result
