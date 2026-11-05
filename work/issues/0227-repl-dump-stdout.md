# 0227: REPL `.dump` without an argument prints the dump to stdout

status: open
agent: -
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

## Follow-ups

## Result
