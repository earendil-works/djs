# 0172: Exporter: continuations, re-enactment, and the dump-every-idle stress mode

status: done
agent: portable
branch: issue/0172-dump-exporter
depends-on: 0171, 0173
touches: crates/djs-core/src/vm/dump*.rs, crates/djs-difftest (--dump-every-idle), crates/djs-core/tests/e2e.rs (dump mode), scripts/gate.sh (the new stress leg), vm/dump_harvest.rs, TESTING.md, docs/continuation-dumps.md, docs/functions-as-code.md
guide: -

## Goal
`Vm::dump` writes format 2 when the VM is idle with something in flight
(docs/continuation-dumps.md), and the gate proves it: at every idle point, dump, load into a
fresh VM, continue, same transcript.

## Acceptance criteria
- [x] Dumping requires an idle VM (D2561) and refuses otherwise with the reason.
- [x] Roots: the host's handles and open host calls, plus the selected bindings; activations
      found through promise reactions and generator objects, written in dependency order with
      the sites and roles from 0169.
- [x] Re-enactment (D2562): promise combinators with settled elements, then/catch/finally
      reactions in order, resolving functions, array/Map/Set/string/regexp-string iterators,
      for-await and async-from-sync iterator state. Anything else in flight is refused with its
      path.
- [x] Unit round trips per construct (each role, each loop kind, each re-enactment) and
      refusal tests.
- [x] difftest `--dump-every-idle` over codemode and codemode-harvest (needs 0173's idle
      points): a transcript that differs from the plain djs run is a hard failure. Added to
      the gate next to `--suspend-every 97`; TESTING.md updated.
- [x] e2e: `DJS_E2E_MODE=dump`.
- [x] Coverage: vm::dump_harvest at every idle point of the harvested cases and of the private
      scripts (counts by refusal reason) in the result. Target: every harvested idle point.
- [x] Measure the dumper's wasm size (feature `dump`) before and after.

## Log
- 2026-11-04 (portable): interfaces (answered for 0179/example and 0174/repl):
  `DumpOptions { base, host, host_fn_name: &dyn Fn(u32) -> Option<String>, handles:
  &[(&str, Value)] }` (Default impl). Handles are host-supplied, written last as
  `$dump.handle(name, v)`; the exporter never adds the entry call's outcome itself (hosts
  pass it, e.g. "run"). `host_fn_name` names open calls (`$dump.pending(id, name, args)`)
  and host functions held as values (`$dump.host(name, isAsync)`, a new loader primitive
  resolved through `LoadOptions::host_fn`). None refuses.
- Script cells keep their compile options (`Script::OPTIONS`, `Script::PARAMS`), so the
  exporter compiles an activation's script again exactly as it was (into the scratch base
  heap; immediates are fixed width, so pcs agree).
- Exporter (vm/dump_flight.rs, a child module of dump.rs): idle check; roots (open calls
  in table order, handles, globals); discovery through pending promises' reactions,
  activations (frames read by the map from compiling the function's script again with
  its stored options into the scratch base heap), builtin closures (resolving functions,
  combinator elements, finally, async-from-sync), Records; creators of pending promises
  and reactions in order with a cursor per promise (D2610). Site comments over every (later dropped, D2616)
  resume (asked by example/0179).
- Found on the way: scope groups could not hold two variables of one name (the corpus
  prelude's per-iteration `impl`/`name`): nested factories (D2611). Frozen arrays lost
  their elements in dumps (elements live in properties): fixed (`array_element`). A
  parameter slot both in `args` and `bindings` was inlined once and empty the second
  time: parameters count as two references now (found by the private scripts).
- Re-enactments: then/catch/finally, combinators (settled elements in place, dead ones
  when the result already settled), resolving functions (`new Promise` + `$dump.rN`,
  `$dump.spent()` when settled), race, async functions returning promises, array/string/
  Map/Set iterators, collections only an iterator holds, `for await` over sync iterables
  (`$dump.fromSync`), unstarted and finished generators, arguments objects, Proxies
  (D2612).
- Tests: tests/dump/v2/*.src.js round-trip at every idle point (plan, roles, generator,
  asyncgen, combinators, loops, cells, steps (example's scenario, also kept as an exported
  golden fixture steps.dump.js), reenact, cycles); refusals (non-idle VM, delivered result,
  unnamed host functions); value round trips for pending promises, generators, Proxies,
  frozen records.
- Gate: difftest `--dump-every-idle` in the main djs step (1,194 idle points, all
  identical, ~15 CPU-s debug); e2e dump mode in the default modes (52 round-trip, 11
  refused); TESTING.md (D2613).
- Coverage (`vm::dump_harvest::idle_harvest_coverage`/`idle_extract_coverage`): harvest
  1,164 idle points in 288 cases, all dumped/loaded/equal; private scripts (1,312 cases
  regenerated with scripts/harvest/build.py from work/scratch/0130/extract.jsonl into
  work/scratch/0172/private, so their fakes answer through `later`; the 0167 extraction
  has no idle points): 5,257 idle points in 1,260 cases, all dumped, loaded and equal.
- After rebasing on 0174/0179: tests/corpus/ts/codemode-tools.ts has idle points now; its
  functions are TypeScript text: dumps holding them are `// djs-dump 2 typescript`
  (D2615). Frame boxes no longer join scope groups through the frame (only through
  closures); a frame's hidden binding shared with closures (`<fields 0>`) is refused.
  The exporter round-trip tests treat `later`/`print` as host globals, as `djs resume`
  (0174) does; steps.dump.js regenerated so `djs resume` loads it.
- Owner: no comments in the format. The site comments, the "In flight" header line and
  the generator comment lines are gone; `Vm::dump_report` returns the sites instead
  (D2616), tested on example's scenario (the_report_names_where_activations_stopped);
  steps.dump.js regenerated without them.
- Size (D2614): dump module 507,129 -> 537,056 gz; default 443,610 -> 443,813 (Script
  options). Docs: continuation-dumps.md "The exporter as built", functions-as-code.md
  refusal table; guide: none affected.
- After the merge, two blockers (branch issue/0172-async-body-source): (1) example
  (0179): a codemode body run's own activation was refused ("a function without source
  text"). The exporter writes an async body as `async function (tools, console)
  {\n<body>\n}`, takes the wrapper's map site after checking it equals the body's
  (refuses otherwise, e.g. `arguments`), report positions stay the body's (D2617);
  tests a_codemode_body_run_round_trips (every idle point, run's promise as a handle,
  report line/col), a_codemode_body_that_differs_as_a_function_is_refused. (2) repl
  (0174): a global lexical binding with a host name (plan.src.js `const tools`) was taken
  for the host's value and dropped; now dumped, and the checker refuses free names that
  would switch from a script binding to a host/base global, and captures read as host
  globals unless they hold its value (D2618); the every-idle-point test now uses hosts
  `console, tools, print, later`, plan included. Docs: continuation-dumps.md "The
  exporter as built"; guide: none affected.
- repl (0174): a body whose parameter a closure captures (`function f() { tools; }`,
  tests/run/todos.js) was refused by D2617's check: the body put the boxed parameter
  before `<state>`, an async function after. The parser starts a body's async flavor
  before its parameters now (D2620); tests a_codemode_body_whose_parameter_a_closure_captures
  (the repro's maps equal, a run dumped at both idle points) and djs-cli's
  dump_after_every_idle_point with todos.js --body; using_body.smap re-pinned.
- example (0179): `const r = await ...` pending wrote `r: undefined`. Checked: that is a
  slot the compiler leaves without TDZ because no read can precede the declaration
  (unobservable); a captured `r` is a cell in its TDZ. The real bug was the other case:
  a slot the compiler keeps in its TDZ (`const r = (await x) + r`, a loop reading `r`
  before its declaration) was refused as "an internal value"; now `$dump.TDZ`. Test
  bindings_in_their_tdz_at_an_await.
- example (0175): classes with instance fields declared in a waiting function were
  refused at the frame's `<fields 0>`. Class hidden bindings in frames are written as
  undefined (dead after the statement); a private name as a value was written as a
  fresh `Symbol("#n")` (found by the FIELDS case: `this.#n += await` resumed into a
  TypeError) and is now `$dump.private(C, "#n")` (D2621). Tests
  classes_with_fields_in_a_waiting_frame (5 bodies, every idle point),
  a_private_name_operand_is_the_class_s; js/test/dumpresume.test.ts's class-fields case
  is no longer a todo.

## Follow-ups
- Computed class member keys (`[k] = 1`, `[k]() {}`) are evaluated again where the dump
  writes the class text: `k` resolves there (a ReferenceError at load, or another `k`:
  silent). Needs a refusal (detect computed keys) or writing keys by value (D2621).
- for-in loops in activations; sites the maps call unsupported (finally, using,
  destructuring, yield*, closing async iterators); iterator helpers, RegExp string
  iterators; allKeyed; async generators awaiting a `return()`.
- The VM's clock and Math.random state are not in dumps (the host sets them again).
- Trim the dumper's size (+29.9 KB gz in the dump module).
- A dump per idle point copies the base image (Dumper::new) and compiles twice (maps,
  checker): ~13 ms per idle point in debug; cache the base copy if the leg gets slow.
- Sandbox/wasm (0175/0179): `vm_dump` needs host_fn_name and handles staged, `vm_load_dump`
  host_fn (example has it on its branch).

## Result
Landed: `Vm::dump` writes what is in flight (format 2): open calls, pending promises with
their reactions in order, suspended activations (by the suspension maps, with a site
report entry each, `Vm::dump_report`), generators, iterators, Proxies, arguments objects; `DumpOptions {
host_fn_name, handles }`; `$dump.host`. Gate: difftest `--dump-every-idle` over the corpus
(1,194 idle points, identical), e2e dump mode. Coverage: every harvested and private idle
point dumps, loads and continues identically. Known gaps: see Follow-ups.
