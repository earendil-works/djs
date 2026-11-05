# 0221: `debugger;` pauses in the CLI: REPL and `djs run`

status: in-progress
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

## Follow-ups

## Result
