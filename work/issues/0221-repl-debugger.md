# 0221: `debugger;` pauses in the CLI: REPL and `djs run`

status: done
agent: dbg
branch: issue/0221-repl-debugger
depends-on: 0220 batch 1 (D3050, on main)
guide: the CLI/REPL page (coordinate with 0220's agent `guide`, who writes it)
touches: crates/djs-cli/ (repl/, run.rs, dumps.rs, tools.rs), CLI tests and fixtures, docs/cli.md

## Goal (owner)
The same `debugger;` pause the docs site's live examples have (D3050: compile option
`debugger_awaits`, `debugger;` in an async function is an await of the host's `debugger`,
identity "debugger"), in the CLI:

- **REPL:** in `djs repl --frozen` (the dumping mode), `debugger;` inside an async function
  or at the top level of an input (inputs are async bodies) pauses the input: the REPL prints
  where it paused (line), and lets the user inspect the continuation dump (e.g. `.dump` shows it,
  `.continue` / `.c` resumes the run in place, and resuming from the dump in a fresh VM is
  possible (`.restore`-style), so a reader sees that the dump alone continues the run). Several
  pauses in a row; Ctrl-C / `.abort` drops the paused input. Design the exact commands to fit the
  REPL's existing ones (.dump, .dump-every, .restore, .pending, .explain …) and keep them few.
- **`djs run`:** an opt-in flag (e.g. `--debugger`) where a `debugger;` pause writes the dump
  (to `--dump-out` or a file next to the script) and either stops (resume later with
  `djs resume`) or continues; pick what fits the existing `--dump-after`/`resume` flags.
- Without the REPL's dumping mode or the flag, `debugger;` does nothing, as today.

Tests: CLI tests that pause, dump, resume in place and from the dump (also a re-dump after a
resume, and a loop with `debugger;` hit several times). Document it in docs/cli.md and tell the
coordinator the user-facing behavior so the guide's CLI page (0220) describes it.

## Log
- Design sent to and approved by the coordinator (abort = `vm.abort()` with a message that
  counts what it dropped; no `.c fresh`; a first-pause hint; key pauses on the identity).
- A pause is an open host call of `debugger` (fixed id 8, identity `debugger`); the CLI
  tells it by identity (`Tools::is_pause`). Dumps' loader compiles with
  `debugger_awaits` in the debugger modes; `$djs.source` tries the option too.
- REPL (`--frozen`): inputs with a top-level `debugger;` (token scan,
  `input::has_top_level_debugger`) run as top-level-await inputs; `[paused]` line with
  source and caret, `.continue`/`.c [id]`, `.abort`, Ctrl-C on an empty prompt aborts a
  pause; `.restore` of a dump with a pause announces it as `[paused]`.
- `run`/`resume --debugger OUT`: dump at the first new pause and stop; resume answers the
  reopened pause at once.
- Checked: fmt, clippy, djs-cli tests, djs-docs check. The plain-dump fuzzer runs
  djs-difftest, not the CLI, and djs-core's dumper is untouched, so not run.
- Guide pages: none changed here; the CLI page (0220) should describe the behavior
  (sent to the coordinator).

## Follow-ups

## Result
D3060. `djs repl --frozen`: `debugger;` pauses (`[paused] #id input N at repl:L:C`),
`.c [id]` continues in place, `.dump`/`.restore`/`.c` continues from the dump, `.abort`
(or Ctrl-C on an empty prompt) drops everything in flight. `djs run|resume --debugger OUT`:
dump at the first pause and stop; `resume OUT` continues, `resume --debugger OUT OUT`
steps. Documented in docs/cli.md; tests in crates/djs-cli/tests/it/{repl,run}.rs.
Known gaps: the `[paused]` line of a top-level pause shows the rewritten input (`0,  n =
1; debugger;`), as `.bt` does for top-level-await inputs; such inputs print no completion
value.
