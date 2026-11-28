# 0312: Psychoscript in the engine benchmarks

status: in-progress
agent: ppsycho
branch: issue/0312-bench-psychoscript
depends-on: -
touches: js/bench/engines/ (engines.ts, run.ts, bridge.ts, ops.ts if it fits, package.json, package-lock.json, prelude.js only if a workload needs it), scripts/bench-engines.sh, scripts/status.py, tests/scores/status.tsv, crates/djs-docs (status.rs gen:status tables/charts, only as needed for a new engine column), docs/guide/benchmarks.md, docs/decisions.md
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
