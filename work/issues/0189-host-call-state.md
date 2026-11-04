# 0189: Host calls as data: per-call state, save/resume hooks, run.suspend, sleep example rewrite

status: done
agent: hostcalls
branch: issue/0189-host-call-state
depends-on: -
guide: docs/guide/host.md, suspension.md
touches: crates/djs-core/src/vm/dump*.rs (pending gets a state argument; Loaded.calls[].state), crates/djs-wasm (vm_dump/vm_load_dump carry per-call JSON), js/src/sandbox.ts (function descriptors {call, save, resume}, run.suspend(), Sandbox.resume(...).result), js/test, examples/sleep-dump (rewritten, ~40 lines), docs/host.md, guide host.md

## Goal
docs/suspension-design-review.md, section 4 "Migration plan", step 1; decisions D2720
(state machines in the compiler, SM), D2721 (slot naming construct@span), D2722 (host-call
state with save/resume hooks). Read the whole report first: it describes this step's
design, what stays and what goes, the risks and the tests required at every step:
smap pins and golden dumps (rewritten only with a decision, D2619), DJS_E2E_MODE=dump,
difftest --dump-every-idle, the harvested/private idle-point counts at 100%
(vm::dump_harvest), test262 async/generator suites where relevant, benches before/after
where the report says so. Each step merges green on its own.

## Log

- 2026-11-04 (hostcalls): engine: `DumpOptions::call_states` (id → JSON text), checked by
  vm/dump_state.rs (JSON only, no `__proto__` key: an object literal reads it as the
  prototype) and written verbatim as `$dump.pending`'s fourth argument; the loader
  stringifies it back into `OpenCall::state`; a state for a call that is not open is
  refused. wasm: `vm_dump` staging gets a fifth line `id:bytes` plus the texts,
  `vm_load_dump` a state length per call and the texts after the handles' names; ABI 3
  (sandbox.ts, js/pi/engine.ts follow). D2770.
- Sandbox: `AsyncHostFunction<S>` `{call, save?, resume?}`, `HostCall` `{id, fn, args,
  startedAt, state, signal}` (also `this` of `call`); `sb.start()` → `Task` with `result`,
  `calls`, `suspend()`, `checkpoint()`; `Sandbox.resume(dump, opts)` / `sb.resume(dump)`
  → a driven `Task` (resume hook, else `call` again, plain functions rejected). The old
  host-driven `Sandbox.resume` is now `sb.loadRun(dump)`; `Run.dump(states)`; loaded
  `OpenCall.state`. D2771. Private `start` renamed `enter`.
- examples/sleep-dump: 77 lines in all, of which the script is 14 and the host ~45 (the
  tool with its hooks, `suspend`/`resume`, the CLI); `run.sh --restart` runs the halves
  as two processes; process 1 exits at once (the call's signal clears its timer); the
  resumed sleep waits only for the rest of its deadline (10.2 s in all for 2×5 s).
  README with the real dump.
- Tests: vm::dump_state (validator, dump→load round trip, refusals), js/test/task.test.ts
  (tasks in packagings A and both B's: plain run, suspend/resume through hooks twice,
  checkpoint, failed dumps keep the task running, dispose, host-driven states),
  sleep-dump.test.ts rewritten (no longer asserts `call.this`/`for-of`, which sm's slots
  change). JS suite 128/128, djs-core vm::dump, djs-cli green.
- Docs: docs/host.md (Tasks section, host-driven runs, ABI rows and version),
  docs/continuation-dumps.md (pending row, open host calls, Sandbox API), guide host.md
  ("Persisting a run that waits" rewritten around tasks), guide snapshots.md (pending
  bullet, where to look). suspension.md unaffected (in-process suspension only).
- Found: `cargo test -p djs-wasm --features compiled-prelude` fails
  `base_equals_a_compiled_realm` without this change too (not a gate leg).

## Follow-ups

- examples/sleep-dump/README.md shows the dump with `operands`/`loops`: regenerate it when
  0191/0192 change the frame format (`node examples/sleep-dump/sleep-dump.ts both 5000`).
- The CLI and REPL (`djs run --dump-after`, `.dump`, `resume --tools`) write no call
  states; `resume --explain` shows them.
- djs-wasm `base_equals_a_compiled_realm` fails under `--features compiled-prelude`
  (pre-existing; no gate leg runs it).

## Result

Open host calls carry the host's JSON state through dumps:
`$dump.pending(id, name, args, state)`, `Loaded.calls[].state`, `vm_dump`/`vm_load_dump`
(ABI 3). The Sandbox drives runs as tasks: `sb.start(code, params)` → `task.result`,
`await task.suspend()` (a dump at the next idle point, `save` per open call),
`(await Sandbox.resume(dump, opts)).result` (each open call continued by `resume(state,
call)`, else `call` again, else rejected). Async host functions are plain functions or
`{call, save, resume}`. The host-driven API stays (`begin`/`settle`/`advance`,
`Run.dump(states)`, `sb.loadRun(dump)`). examples/sleep-dump is rewritten on tasks.
