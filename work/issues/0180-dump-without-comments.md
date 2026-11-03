# 0180: The dump format without comments

status: open
agent: -
branch: issue/0180-dump-no-comments
depends-on: 0172 (lands first; both edit vm/dump*.rs)
guide: -
touches: crates/djs-core/src/vm/dump*.rs (writer and the loader's header detection), tests/dump/ (fixtures), crates/djs-cli (REPL/run/resume if they print or expect comments), js/src/sandbox.ts and js tests (if they check text), examples/sleep-dump (README), docs/functions-as-code.md, docs/continuation-dumps.md, docs/cli.md

## Owner
"Change the JavaScript dump format to not include the comments."

## Goal
A dump contains no comments: not the explanatory header lines ("N bindings …", "Run it as
a Script…"), not the section comments ("Builtins the session changed.", "Links that close
cycles, and attributes.", "<name> sharing <vars>"), and not 0172's site comments above
`$dump.resume` (source line + caret).
- The **version marker** `// djs-dump N` is a comment today and the loader detects the
  format by it. Replace it with a non-comment marker (e.g. a directive prologue
  `"djs-dump 2";` as the first statement, or a `$dump.version(2)` call for format 2),
  propose the choice to the coordinator before changing it; version 1 dumps (D2352's
  promise, tests/dump/v1.js) must keep loading (D024: no compat for the *new* format
  needed, but v1's promise stands unless the owner drops it; ask).
- What the comments explained moves out of the format: e.g. `djs heap`/the REPL or a small
  `djs dump --explain` (or `Vm::describe_dump`) can show the suspended sites with source
  lines and carets for humans (the sleep-dump example uses that instead of comments).
  Propose the smallest form.
- Tests, fixtures, docs and the example updated.

## Log

## Result
