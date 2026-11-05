# 0227: REPL `.dump` without an argument prints the dump to stdout

status: done
agent: repld
branch: issue/0227-repl-dump-stdout
depends-on: -
guide: cli.md (REPL commands)
touches: crates/djs-cli/src/repl/, CLI tests, docs/cli.md, docs/guide/cli.md

## Goal (owner)
In `djs repl`, `.dump` with no argument fails ("unknown command or missing argument: .dump").
It should print the dump to stdout (what `.dump -` does today). Also:
- if the REPL can't dump (not started with `--frozen`), say so clearly ("dumps need frozen
  builtins: start the REPL with `djs repl --frozen`") instead of "unknown command";
- check the other dump-related commands (`.dump-every`, `.restore`, `.explain`, `.c`) for the same
  kind of unhelpful missing-argument error and give them sensible no-argument behavior or a usage
  line;
- `.help` shows the forms.
CLI tests for each; update docs/cli.md and the guide's cli.md.

## Log
- `.dump` with no argument prints the dump (same as `.dump -`). Without `--frozen`,
  `.dump`/`.dump-every on` say "dumps need frozen builtins: start the REPL with
  `djs repl --frozen`".
- No-argument forms: `.bt` = `.bt all`; `.restore`, `.disasm`, `.load`, `.resolve`,
  `.reject` print a usage line; `.c` already continued the oldest pause; `.dump-every`
  shows the state. There is no REPL `.explain` (it is `djs resume --explain`).
- Unknown commands now say "unknown command: .x" (the command only).
- Paused hint says `.dump shows the continuation`; `.help` shows `[file]`/`[id|all]`.
- Updated docs/cli.md and docs/guide/cli.md (REPL command tables, dumps section).

## Follow-ups

## Result
`.dump` prints to stdout without a file; dump commands without `--frozen` say how to fix
it; every command with a required argument prints its usage when called bare; `.bt`
defaults to `all`. Test: `repl::commands_without_arguments`.
