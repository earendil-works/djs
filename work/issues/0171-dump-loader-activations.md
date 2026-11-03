# 0171: Loader: suspended activations, open calls, handles

status: done
agent: portable
branch: issue/0171-dump-loader
depends-on: 0169, 0170
touches: crates/djs-core/src/vm/ (load_dump primitives), interp/run/suspend.rs (building a Generator from a description), host call table, tests/dump/v2/ (hand-written fixtures), docs/continuation-dumps.md
guide: -

## Goal
The loading side of format 2 dumps (docs/continuation-dumps.md "The loader primitives",
D2562), tested on hand-written dumps before any exporter exists.

## Acceptance criteria
- [x] `$dump.resume(fn, {at, awaiting, this, args, bindings, operands, loops})` builds a
      suspended activation of `fn` at the named site from this build's suspension map (0169),
      subscribes it to `awaiting` and returns its result promise; generators and async
      generators at `yield#N` return the generator object (and an async generator's request
      queue).
- [x] It refuses, with the reason, a site that does not exist, a binding or role or loop the
      map does not have or lacks, a value of the wrong kind (a non-promise `awaiting`).
- [x] `$dump.cell(() => x)`, `$dump.pending(id, name, args)` (an entry in the host call
      table with the host's id), `$dump.handle(name, v)` (readable by the host after the load),
      `$dump.TDZ`.
- [x] After the Script, the loader drains the job queue and reports the VM idle with its open
      calls and handles.
- [x] Hand-written format 2 fixtures in tests/dump/v2/: at least the plan's example, a for-of
      with an await, every operand role, a generator, an async generator, nested activations,
      Promise.all/allSettled/any/race in flight, a rejection path. Each loads, the test
      delivers results, and the output equals the uninterrupted run's.
- [x] Format line `// djs-dump 2`, and a decision recording the format 2 promise.

## Log
- 2026-11-04 (portable): read the plan, D2560–D2565, D2570, suspend.rs, promise.rs, the host
  call table. Design (sent to the coordinator): `load_dump -> Result<Loaded, LoadError>`
  with the open calls and handles; `LoadOptions { fuel, host_fn }` maps a pending call's
  name to the host's function id; resume's description as in docs/continuation-dumps.md
  (operands as `[role, value]` pairs since roles repeat).
- vm/dump_resume.rs (new): `$dump.resume`. The maps come from compiling the dump's text
  again (once per load) and the map with `fn`'s span; `fn` must be from that script.
  Frame = callee, this, args (padded), header (argc), locals, operands, in a Generator
  object made like generator_start/async_start do (suspend.rs `new_activation`,
  `set_suspended`), subscribed to `awaiting` with `perform_then(p, g, g)` as %AsyncAwait.
  Captured bindings need a Box: compiler/smap.rs `Site::boxed` (from resolve.rs Storage;
  pins unchanged; also: a local slot shared by bindings of disjoint scopes keeps the one in
  scope). `Role::parts()` for role names without core::fmt.
- vm/dump_load.rs: `cell`, `pending`, `handle`, `issued`, `TDZ` (TDZ and cells are internal
  Records with one slot: UNINITIALIZED or the Box). Found: the primitives' own (parked)
  calls take ids from the counter and collided with `pending(1, …)`; while loading they now
  use ids from 0x8000_0000, and the counter is set at the end (issued, else as before,
  past every open id). interp/run.rs `add_loaded_host_call`.
- Fixtures tests/dump/v2/{plan,roles,generator,asyncgen,combinators,loops,cells}: program
  + hand-written dump at idle point K; the test runs the program (later settled oldest
  first at idle points), loads the dump into a fresh VM, continues, compares; again under
  gc stress in 97-fuel slices. Refusals: unknown site, missing/extra binding, wrong/missing
  /extra operand, extra loop, bad pair, non-promise awaiting, awaiting at a yield, unknown
  key, holes, cell for an uncaptured binding, queue on a non-async-generator, not async,
  unsupported site (finally), for-in, a function not from the dump's text, cell of two
  variables, duplicate/unknown/invalid pending, duplicate handle. `show_maps` (ignored)
  prints a file's maps for writing fixtures.
- Decisions D2583 (the format 2 promise), D2584 (resume), D2585 (calls, handles, ids).
  docs/continuation-dumps.md: the example's operands, "As built (0171)". Guide: no page
  affected.
- Size (dump module, default wasm + dump, gzip -9, same toolchain): 498,277 -> 507,534
  (+9.3 KB; +25 KB raw, mostly load_dump with the primitives inlined, resume_operands,
  resume_queue). Default and slim modules unchanged (all behind `dump`).

## Follow-ups
- for-in loops in `resume` (the key iterator is internal; needs a description).
- djs-wasm `vm_load_dump` / `Sandbox.load` pass default LoadOptions: a dump with open calls
  refuses there (no host_fn), and the open calls and handles are not returned over the
  ABI (0175).
- REPL `.restore` ignores `Loaded` (its handles stay rooted); 0174 uses them.
- The exporter (0172) needs the inverse of host_fn (fn_id -> name), and must give the
  `arguments` hidden local when a function uses it.

## Result
Landed: the format 2 loader's activations. `$dump.resume(fn, {at, awaiting, this, args,
bindings, operands, loops, queue})`, `$dump.cell(() => x)`, `$dump.pending(id, name, args)`,
`$dump.handle(name, v)`, `$dump.issued(n)`, `$dump.TDZ` (docs/continuation-dumps.md "As
built (0171)", D2583–D2585). `Vm::load_dump(text, &LoadOptions { fuel, host_fn })` now
returns `Loaded { calls: Vec<OpenCall { id, fn_id, name }>, handles: Vec<(String,
Handle)> }`; the VM is idle after it, waiting on those calls (already reported). Fixtures:
tests/dump/v2/*.src.js + *.dump.js, run by `vm::dump_resume_tests` (plain and gc stress).
Known gaps: for-in loops, and every site the maps call unsupported, refuse; wasm/Sandbox
and the REPL do not use the new report yet.
