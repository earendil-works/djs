# 0174: REPL and CLI: tools, waiting inputs, continuation dumps

status: in-progress
agent: repl
branch: issue/0174-repl-continuations
depends-on: 0171 (the tools and pending-call part can land before it)
touches: crates/djs-cli/src/repl/, crates/djs-cli/src/run.rs, crates/djs-cli/src/main.rs, crates/djs-cli/tests/, docs/cli.md
guide: -

## Goal
Make suspension visible and testable by hand (docs/continuation-dumps.md "REPL").

## Acceptance criteria
- [x] A host-provided `tools`: `tools.x(args)` is an async host call answered by hand;
      inputs that wait do not block the prompt and print their result when they settle.
- [x] `.pending`, `.resolve <id> <json>`, `.reject <id> <message>`, `.bt <id|all>`
      (suspended activations and their bindings); `--tools answers.json` for scripted answers.
- [x] `.dump` writes format 2 when something is in flight; `.restore` restores waiting inputs
      and open calls; `.dump-every [on|off]` dumps and reloads at every idle point.
- [x] `djs run --tools answers.json --dump-after K out.js`, `djs resume out.js --tools
      answers.json`, `djs run --dump-every-idle`.
- [ ] Line-mode transcripts in crates/djs-cli/tests/repl.rs for each command; a CLI test that
      runs `--dump-after K` / `resume` for every K of a few scripts and compares with the
      uninterrupted run. docs/cli.md updated.
- [x] `.snapshot` stays until 0177.

## Log
- 2026-11-03 (repl): tools part. `tools` is a Proxy that makes one async host function
  per name through a sync `make` host call (D2600); `crates/djs-cli/src/tools.rs` (new
  file, shared by repl and run) has it, the answers file (D2601, a small JSON reader:
  djs-cli has no serde) and JSON helpers. REPL: waiting inputs (kept as handles on their
  promises, printed `[input N] value` when they settle), `[waiting] #id ...`,
  `.pending`, `.resolve`, `.reject`, `.bt <id|all>` (repl/activations.rs reads Generator
  objects: location, parameters named from the function source, locals; the chain from a
  call's promise through await reactions, `then` capabilities and combinator element
  records). `djs run --tools` answers at idle points, exit 1 for a call with no answer.
- Outside touches, coordinated: `Vm::outcome()` (vm/mod.rs, +test), the entry call's
  value while `run` stops Idle; `crates/djs-cli/src/heap/mod.rs`: `json` module made
  `pub(crate)` (the J type); `crates/djs-cli/src/tools.rs` new.
- Tests: tests/repl.rs (tools_wait_and_resolve, backtraces_of_waiting_activations,
  answers_file), tests/run.rs (todos script with answers, plain/--suspend-every/
  --gc-stress; missing answer). A tools transcript is identical under --gc-stress and
  .suspend-every 50. Guide: no page affected.
- Merged the tools part (main a54e0504). Prepared for 0171's `LoadOptions::host_fn`:
  `Tools::id_for(name)` (a name keeps its id; `make` reuses it), `id_for_call_name`
  (`tools.read` → id, None for other names) and the inverse `call_name(fn_id)` for 0172's
  exporter; names in a RefCell since `host_fn` is a `&dyn Fn`.
- 0171 landed. Dump part, what the loader allows (D2602, D2603): `djs resume
  [--tools] dump.js`; REPL `.restore` of a format 2 dump restores open calls (ids kept,
  tools by the registry) and the dump's handles as waiting inputs (`[run] value`), no
  rescan of tool names after a load; host globals `later(v, rejected)` (answered with v
  at idle points) and `print` in run/resume/repl so the loader's fixtures run. `.dump` of
  a waiting session, `.dump-every`, `run --dump-after`, `run --dump-every-idle` refuse,
  naming the 0172 exporter. Tests: tests/run.rs resumes every tests/dump/v2 fixture
  (plain and --suspend-every 13 --gc-stress; the output is the tail of the program's
  run), a tools-based hand-written dump (tests/run/reads.*) with answers; tests/repl.rs
  restores reads.dump.js (.pending, .bt, .resolve) and plan.dump.js, and the exporter
  refusals.
- For 0172's `$dump.host(name, isAsync)`: the load lookup also accepts `print` and the
  `console` methods (the printing function's id 1); `call_name(1)` is `print`, `make`
  (id 2) has no name. Committed on the branch, to land with the post-0172 work.
- 0172 landed. Dump commands (D2604): `run --dump-after K OUT` / `--dump-every-idle`
  (idle point = a stop with only host calls open, before answering; the run's async-body
  promise is the handle `run`), REPL `.dump` of a waiting session (waiting inputs as
  handles by label, "wrote N bytes (A activations, C pending calls)"), `.dump-every`
  (round trip at every idle point, the running input's result through the handle
  `$entry`; a refusal prints `[not dumped]` and turns it off). Dumps detected by
  `dump_format`, not the comment prefix. New djs-cli/src/dumps.rs. Tests: every K of
  reads.src.js and of every tests/dump/v2 program except plan (exporter drops a script's
  own `const tools` shadowing the host global; reported), --dump-every-idle equal; REPL
  transcript dump/restore/bt/resolve/dump-every and a refusal.
- Not dumpable yet, waiting on portable's core fix (async-body functions get source
  text): suspensions in the top level of `run --body` (tests/run/todos.js) and the REPL's
  await inputs. The CLI wrapper idea was dropped (coordinator).
- Left for after 0172 (done above, except the async-body cases): format 2 `.dump` of a waiting session, `.dump-every`, `run
  --dump-after K` / `--dump-every-idle`, and the every-K dump/resume CLI test.
- Earlier note: waiting for 0171 for the dump part (.dump format 2, .restore of waiting inputs,
  .dump-every, run --dump-after/--dump-every-idle, resume).

## Follow-ups

## Result
