# 0304: Remove continuation dumps

status: done
agent: psnap
branch: issue/0304-remove-dumps
depends-on: 0301, 0302, 0303
touches: everything dump: crates/djs-core/src/vm/dump*.rs, call_site.rs, compiler/smap*.rs and the dump cfgs in the compiler/interp/heap/builtins, djs-wasm dump exports, js/src (dump-runtime, sandbox dump paths), djs-cli/djs-difftest/djs-docs dump parts, scripts (dump-*), tests/dump, tests/smap, gate legs, docs/guide (dumps.md, continuations.md, suspension.md's persistence part), docs/continuation-dumps.md, TESTING.md, ARCH.md, docs/decisions.md
guide: docs/guide/SUMMARY.md, dumps.md, continuations.md, suspension.md, snapshots.md, js-api.md, cli.md, host.md, pi.md, goals.md

## Goal
With snapshots the persistence format (0301-0303), delete continuation dumps in one issue:
the dumper (~26.6k lines with tests), suspension maps (~2.7k lines plus ~310 recorder calls
in the parser and ~1k lines of dump-only compiler code), the `$djs` hooks and dump runtime,
the dump ABI, the JS paths, the CLI/REPL/difftest parts, the scripts, tests/dump and
tests/smap, the gate legs, `djs_optsize` if nothing else needs it, and the guide pages.
Rules that existed for dumps lapse (origin tracking, builtin state re-enacted, the refusal
taxonomy, statement state in named temporaries as a *requirement*: keep the lowering itself).
No compatibility code (owner: earlier releases need not load).

## Acceptance criteria
- [x] Nothing of dumps left (grep for dump, smap, $djs, continuation in code, tests, scripts
      and docs; what stays has a reason in the log).
- [x] One guide page on persistence (snapshots), written for how djs is now; the guide's
      pages that described dumps rewritten or removed; ARCH.md and TESTING.md follow.
- [x] Size: the shipped module's gz size measured before and after; the ceiling lowered to
      the new size.
- [x] Decisions recorded (what was removed, superseding the dump decisions).

## Log
- Size before (main a87745f2 + the status commit): 785,187 B gz (1,865,016 raw).
- JS: Sandbox without dumps (Run.dump, sandbox.dump/load, loadRun, the `$djs` hooks but
  HOOK_MISSING, scriptHash, plainDump, inlineSources, LoadedDump, LoadRunOptions); a run's
  result is always the entry call's outcome (the `run` handle only came from dumps), so
  snapshots of a sandbox hold no handles. `resume` takes a Uint8Array. A snapshot refuses a
  state for a call that is not open (the dump did). Tests converted to snapshots
  (durable, run, task, host-values*, bindings, imports, sandbox); dumpload, dumpresume,
  plain, golden-host deleted; the package's smoke test and dump runtime exports gone.
- CLI: run/resume without --dump-after/--dump-every-idle/--explain; resume reads only
  snapshots; the REPL without .dump/.dump-every; `--debugger` of its own (pauses), `--frozen`
  only the builtins (decision D3731). The tests/dump programs moved to
  tests/snapshot/programs (golden/combinators -> combinators-2); the CLI's every-idle-point
  test runs them all through snapshots.
- djs-docs: `gen:dump` -> `gen:snapshot` (the snapshot described); no engines-dumps table.
- difftest: no --dumps/--dump-out/--dump-from, no `dump-xfail`.
- djs-wasm: ops vm_set_dump_source_refs, vm_dump_runtime, vm_dump, vm_open_call,
  vm_restore_math_random, vm_set_next_call_id gone, renumbered, ABI 11. The shipped
  module had `lazy_functions` only through `dump`: measured without the dumper, lazy costs
  +5.0 KB gz (515,646 vs 510,652) and changes no codemode-memory row, so the shipped module
  compiles eagerly (D3732). Modules with snapshots keep the 128 KiB stack the dump module
  had: a snapshot's load compiles the scripts without the host (no elastic retry).
- djs-core: vm/dump*.rs (26.5k lines), compiler/smap*.rs (2.7k), the ~310 parser hooks,
  the dump-only compiler code (compile_with_maps, free_names, class spans, lazy globals,
  the scans' early stop, ArgsOf, `stack_names`), `Vm::open_host_call`,
  `restore_math_random`, `set_next_call_id`, `script_hash`, `source_function`,
  dump_runtime, the base64 fast path, `private_entries`, `registry::base_globals`,
  `sort_cold`, `Suspended`'s unused fields; the temporaries keep their lowering but lose
  their names (a count). Bytecode unchanged (frame pin passes). `djs_optsize` gone.
- `Vm::call_site` and `read_activation` stay: they read activations (the Sandbox's
  `callSite`, the docs site's pauses), no dump code.
- djs-prelude: no `names` feature (BASE_NAMES/BASE_GLOBALS).
- Gate: no "dumps (djs and node)" leg; features without `dump`; RUST_KEEP without the dump
  runtime. Scripts: dump-costs, dump-fuzz, dump-node.mjs, dump-size gone; the `scores`
  merge driver (only dump-size.tsv used it) and the smap summary's regen gone;
  merge-driver-test checks the generated driver; status.py without dumps.
- Guide: one page on persistence, docs/guide/persistence.md ("Snapshots: keeping a
  waiting run", in "Using djs": a live `debugger;` example with its `gen:snapshot`, idle
  points, taking and loading, contents, code compiled again and checked by hash,
  scripts by text or hash, host values, later builds, what cannot be snapshotted,
  describe/callSite, pauses). dumps.md, continuations.md and gen/chart-engines-dumps.svg
  deleted; rewritten: suspension.md ("Persisting a waiting run", gen:snapshot),
  snapshots.md ("Beyond the process"), js-api.md, cli.md (REPL `--debugger`/`--frozen`),
  host.md (no dump ops; the shipped module's 128 KiB stack; one module with snapshots),
  pi.md (the durable mode on `run.snapshot()`/`loadSnapshot`), goals.md, index.md,
  introduction.md, language.md, limits.md, determinism.md, builtins.md ("Snapshots and
  changed builtins"), compiler.md (lazy: the wasm modules leave it out), interpreter.md,
  rust-api.md, benchmarks.md (no Dumps section), testing.md (a snapshots section instead
  of dumps), status.md, README.md (gen:snapshot, the live pause text). The live tip links
  persistence.html. `djs-docs check` ok.
- docs/: continuation-dumps.md deleted; cli.md and host.md rewritten for snapshots (both
  had never described them); compiler.md, web-globals.md; status lines of the research
  reports heap-snapshots-revisited.md and suspension-design-review.md say they are
  historical; decisions-index.md lists D3700, D3730-D3734 and the supersession.
- ARCH.md: G4, "Images and snapshots" (was "Images and dumps"; djs-prelude's two
  references follow), the execution API, durability, the stress mode, phases, risks.
  TESTING.md: no --dumps/fuzzer/golden dumps/e2e dump mode/smap/dump-size; the programs;
  the merge drivers; size history. README.md, AGENTS.md, work/README.md, the workflows'
  comments.
- Size after (shipped features): 510,652 B gz (1,192,832 raw; code 1,016 KB, data 79 KB
  from 108 KB); ceiling 786,101 -> 511,676; codemode-memory every row 64 KiB lower (the
  smaller data segment), recorded; status.tsv's wasm sizes recorded (wasm/ops 74 -> 60).
- Decisions D3730 (removal, superseding the dump decisions), D3731 (REPL --debugger),
  D3732 (eager module), D3733 (128 KiB with snapshots), D3734 (run result, states,
  gen:snapshot, programs).
- What stays (final grep for dump|smap|$djs|continuation): value "dumps" of the json/wire
  tests and js/test/dump.ts (a canonical value printer), gen-cross/reader-cross; the
  interpreter's return continuations (`Continuation`), template and line continuations,
  UTF-8 continuation bytes, AsyncFromSyncIteratorContinuation; `ThisMap` (smap);
  `$DJS_*` environment variables; other engines' `vm.dump`/`ctx.dump` in js/bench/engines
  and `json.dumps` (monty); scripts/size/areas.py's QuickJS JS_DumpMemory patterns;
  prof::dump (scripts/perf/instr.py), chrome's `--dump-dom`, `DJS_DUMP_META` (test262
  frontmatter); llvm-dwarfdump/objdump. Historical records: docs/decisions.md, the
  research reports, work/issues, work/research (0199's Cargo.toml names the old `dump`
  feature; not a workspace member), the TESTING/ARCH history notes, frozen-builtins.md's
  design table. tests/corpus: 83 cases' comments name the plain-dump fuzzer or dumps as
  their provenance; left as they are, since editing them shifts the source spans that
  tests/snapshot/frame-bytecode.txt pins (and the cases stay valid tests).
- The first merge's gate failed "native vs wasm images": djs-abidrive (native) had
  djs-difftest's `lazy` feature, the wasm module no longer has it, and a lazy build gives
  lazily compilable functions their own constant pools, so the raw images differed (the
  canonical hashes agreed). build:difftest now builds djs-abidrive without `lazy`
  (native-vs-wasm.sh's own build too); D3732 says so.
- Ran: djs-docs and djs-cli tests, e2e (default: plain, GC stress, suspend), the JS tests on
  the dev wasm (110 pass), nesting on the shipped module, fmt, gate legs paths/targets.

## Follow-ups
- The engine comparison (benchmarks.md) lost its dump table; snapshot costs (write and
  load time, size per workload) could take its place (`scripts/status.py engines` and
  js/bench/engines/run.ts).
- The `engines/*` measurements in status.tsv were recorded with the dumper module
  (djs's module size, compile time, memory); re-record with `scripts/status.py engines`.
- tests/corpus comments that cite the dump fuzzer or dump behavior (83 files) could be
  reworded together with a regeneration of tests/snapshot/frame-bytecode.txt.
- The shipped module's 128 KiB stack (D3733): an elastic retry inside the snapshot loader
  would give every instance 64 KiB back.
- pi-mono-djs's durable mode still uses dumps (`run.dump`, `loadRun`); it needs to move to
  `run.snapshot()`/`loadSnapshot` and ABI 11.

## Result
Continuation dumps are gone: the dumper, the suspension maps and their parser hooks, the
`$djs` runtime, the dump ABI (djs-wasm ABI 11), the Sandbox's, CLI's, REPL's, difftest's
and djs-docs' dump parts, the dump tools, tests, gate leg and guide pages. Snapshots
(D3700) are the only persistence format: `task.suspend()`/`checkpoint()`,
`Sandbox.resume`, `run.snapshot()`/`loadSnapshot`, `describeSnapshot`, `callSite`;
`djs run --snapshot-after/--snapshot-every-idle/--debugger`, `djs resume`, `djs
describe`, the REPL's `.snapshot`/`.restore` and `--debugger`. The guide has one page on
it, persistence.md. The shipped module is 510,652 B gz (from 785,187), eager, with a
128 KiB stack; its ceiling is 511,676. Known gaps: see Follow-ups (engine-comparison
numbers stale, corpus provenance comments, the 128 KiB stack, pi's durable mode).
