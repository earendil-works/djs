# 0204: Host hooks for plain dumps: resume(id, name, args, state), seed(lo, hi); Sandbox tasks, pi adapter, CLI/REPL

status: done
agent: pjh, pjm
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
  value; otherwise a null-prototype object of all handles by name (agreed, D2895). REPL
  `.restore`/`--explain` unpack it (ee6b886f).
- After 0201 landed: vm_dump's sixth staging line `plain`; `SandboxOptions.plainDumps`,
  pi `VmHooks.plainDumps` (`PLAIN_FN_NAMES`), `djs run/resume --plain`, REPL `--plain`/
  `.plain` (several handles). Real exporter output through every loader: CLI every idle
  point of tests/dump/v2 and the CLI fixtures (run::plain_dump_after_every_idle_point),
  REPL plain_dump_waiting_await_inputs, js dumpresume's every-idle-point cases in a plain
  variant per packaging, plain.test.ts task test, pi-vm's dumped-session test, pi's whole
  suite `--next --dump-every-idle --plain` (215/226, the same 11; added to the full-only
  scripts/pi-dump-every-idle.sh). Remaining refusals are the exporter's (0203): for-await,
  private fields, "an activation of a factory's cell" (dumpresume `classes`).
- Found: call ids drift after a plain load (the hooks are sync calls taking ids). Fixed
  with `$djs.seed(lo, hi, next)` (exporter: one argument; hosts set the counter once the
  Script stopped; `Vm::set_next_call_id`/`vm_set_next_call_id`), D2897, also in the
  difftest's host. The CLI had a second drift from its lazily made tools: call states
  carry `made`, a load premakes them. Every-idle-point tests compare ids now.
- examples/sleep-dump on plain dumps, with `node` mode / `run.sh --node` continuing the
  dump in node itself; README rewritten. `--explain` hides `$djs_*` scaffolding.
- Docs: docs/host.md (plain dumps, ABI rows, pi), docs/cli.md, docs/plain-js-dumps.md
  (the interface as built). Guide: host.md (plain dumps paragraph), snapshots.md (the
  plain format, short); suspension.md unchanged (0205 rewrites the dump chapters).
- pjm took over: `SandboxOptions.plainDumps` defaults to true (D2898); the js tests that
  assert format 2's text opt into it with `plainDumps: false` (dumpload, task, dumpresume's
  format 2 packagings; 0205 rewrites them). sleep-dump drops its now-default option and
  clamps a passed deadline's timeout at 0 (node warned on the negative delay after a
  resume). Checked: js tests (152 pass), `run.sh`, `run.sh 300`, `--restart 300`,
  `--node 300`, js/test/sleep-dump.test.ts. Docs: docs/host.md, guide host.md and
  snapshots.md (the default); CLI and pi adapter unchanged (format 2 default until 0205).

## Follow-ups

- 0205: make plain the only format: drop `plainDumps`/`--plain`/`.plain`, vm_dump's
  `plain` line, and rewrite the format-2 text assertions (dumpload, task, pi-vm, CLI
  explain fixtures) for plain dumps.
- `--explain` of a plain dump shows the continuation's line (`$djs_R(...)`); mapping it
  back to the original line through `$djs.segs` would read better.

## Result

Plain dumps (format 3, D2830) are written and loaded by every host: `$djs.seed(lo, hi,
next)` and `$djs.resume(id, name, args, state)` (D2890, D2897), several handles as one
object (D2895), call states in the CLI (D2894). The Sandbox writes plain dumps by default
(D2898; `plainDumps: false` for format 2); the CLI asks with `djs run/resume --plain` and
REPL `--plain`/`.plain`, the pi adapter with `VmHooks.plainDumps`. Loading takes either
format and needs no dumper for plain dumps. examples/sleep-dump runs on plain dumps:
`examples/sleep-dump/run.sh [ms]`, `run.sh --restart [ms]` (two processes) and
`run.sh --node [ms]` (process 2 continues the dump in node). Known gaps (0205): format 2
is still the CLI's and pi's default and is not removed; the exporter's refusals (0203:
for-await, private fields, a factory cell's activation); `--explain` shows continuation
lines.
