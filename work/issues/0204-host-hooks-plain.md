# 0204: Host hooks for plain dumps: resume(id, name, args, state), seed(lo, hi); Sandbox tasks, pi adapter, CLI/REPL

status: in-progress
agent: pjh
branch: issue/0204-host-hooks-plain
depends-on: -
guide: docs/guide/snapshots.md, suspension.md, host.md
touches: js/src/sandbox.ts, js/pi/, crates/djs-cli (run/resume/REPL), crates/djs-wasm (exports for loading a plain dump = running a script with hooks), docs/host.md, examples/sleep-dump

## Goal
docs/plain-js-dumps.md section 8 "Recommendation and plan", step 5.

## Context
docs/plain-js-dumps.md (the design, results and plan: read all of it), the prototype in
work/research/0199/ (plain.mjs, normalize.mjs, freevars.mjs, the harness and chain checks),
decisions D2830-D2833, and today's format 2 exporter (vm/dump*.rs, D2610-D2628, D2730-D2735,
D2800-D2802). Acceptance at every merge: dumps run as plain Scripts in djs **and node** and
continue identically to the uninterrupted run over every corpus idle point they cover,
re-dumps stay bounded (cap chains at 50 links), and the existing gate checks stay green.
djs is unreleased (D024): no compatibility code for format 2 once it is retired.

## Log

- Interface agreed with pjx through the coordinator: `$djs.seed(lo, hi)` first,
  `$djs.resume(id, name, args[, state])` per open call, called only while the Script
  evaluates; hosts define `$djs` and delete it after the load; the exporter refuses
  globals `$djs`/`$dump`; completion value = the run's promise (one handle); header
  `"djs-dump 3";` / `"djs-dump 3 typescript";`; `DumpOptions::plain` (0201) and a
  `plain` flag in vm_dump. Host functions held as values are written by the host's name
  as an expression: a host must make each name a global path during the load.
- Step 1 (loading): `Vm::open_host_call(parked, fn_id)` (vm/mod.rs) opens the dump's call
  under its id from the parked `resume` call; `add_loaded_host_call` (interp/run.rs) is
  no longer dump-only and moves the counter past the id. Exports `vm_open_call`,
  `vm_delete`. CLI: `resume`, `--explain`, REPL `.restore` run plain dumps
  (dumps.rs: `begin_plain`, `serve_hook`, `end_load`, `run_plain`); `drive_from` serves
  the hooks while loading and has the `--dump-every-idle` plain reload path ready
  (unwired until 0201). Sandbox: `loadPlain` in the main module (no dump module needed).
  Tests: vm::tests::open_host_call_keeps_the_dump_s_id, djs-cli run::resume_a_plain_dump
  (tests/run/reads.plain.js, by hand), js/test/plain.test.ts. D2890, D2891. docs/host.md,
  docs/cli.md updated; guide pages wait for plain dumps being written (step 2).
- pi adapter: its host functions `bridge`/`call` are not globals; for plain dumps they
  will be named `$djs.bridge`/`$djs.call` (defined in `$djs` while loading), so the
  exporter's name expressions resolve during the load.
- Step 1 merged (f5d33fa4), with the size trade D2892 (the gate failed on the budget first:
  open_host_call slimmed to 874 B, `vm_delete` dropped; the dump deletes `$djs` in its
  final block, pjx's D2862). Step 2 merged (9656d97e): pi's `resumeFromDump` loads plain
  dumps (D2893), tested with a hand-made dump in pi-vm.test.ts.
- CLI call states (0189 follow-up, D2894): `Tools::call_state`/`restore_state`/
  `state_origin`; `dumps::dump`/`dump_names` take an origin callback; `dumps::load` takes
  `&mut Tools`. Tests: run::dumps_carry_the_answer_counts; repl::dump_waiting_await_inputs
  now expects the restored origins.
- Proposed several handles in plain dumps to the coordinator: one handle named `run` → the
  value; otherwise a null-prototype object of all handles by name (waiting for pjx).

## Follow-ups

## Result
