# 0304: Remove continuation dumps

status: in-progress
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
- [ ] Nothing of dumps left (grep for dump, smap, $djs, continuation in code, tests, scripts
      and docs; what stays has a reason in the log).
- [ ] One guide page on persistence (snapshots), written for how djs is now; the guide's
      pages that described dumps rewritten or removed; ARCH.md and TESTING.md follow.
- [ ] Size: the shipped module's gz size measured before and after; the ceiling lowered to
      the new size.
- [ ] Decisions recorded (what was removed, superseding the dump decisions).

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

## Follow-ups

## Result
