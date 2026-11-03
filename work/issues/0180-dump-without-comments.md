# 0180: The dump format without comments

status: done
agent: nocomment
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
- Phase 1 (plan, approved by the coordinator): inventory of every comment the writer
  emitted and every place expecting them (loader header detection, REPL `.restore`,
  `djs resume` usage, `Sandbox.load`, js/test/dumpload, the fixture test's `// at idle
  point K` line, fixtures, docs). Marker: directive prologue; old `// djs-dump 2` refused;
  explanation as CLI-only `djs resume --explain`.
- Phase 2, after 0172 landed (it had already removed the site comments, the "In flight"
  header and the generator comments, and added `Vm::dump_report`, D2616):
  - writer: `HEADER` = `"djs-dump 2";`, `HEADER_TS` = `"djs-dump 2 typescript";`; the
    header lines, both section comments and the factories' "sharing" line are gone.
  - loader: `dump_format` reads the directive (any N; the loader takes 2) and the
    comment only for version 1; `is_typescript` reads the directive. Tests:
    `dumps_have_no_comments`, `typescript_dumps_say_so_in_the_directive`,
    `the_loader_reads_the_directive` (old header refused, plain-Script run fails at
    `$dump`).
  - fixtures: tests/dump/v2/*.dump.js (hand-written) lost all comments; idle point K is a
    parameter of `fixture(name, k)`, the prose went into each `.src.js` header (steps.src.js
    unchanged: a test pins its line numbers; its note is on the test). steps.dump.js
    regenerated (same text minus the comments). crates/djs-cli/tests/run/reads.dump.js
    likewise (its positions moved up one line; repl.rs expectations updated).
  - CLI: `djs resume --explain dump.js` (run.rs `explain_dump`), using the REPL's
    activations module (now `pub mod`), which gained `Activation::source` and
    `describe(label)`: the source line (indentation trimmed, at most 120 chars) and a
    caret; `.bt` prints through it. `run::is_dump` for `.restore`.
  - js: `Sandbox.load` accepts either marker; dumpload.test.ts checks the directive and
    no comment lines.
  - docs: functions-as-code.md (example now format 2 output, format section),
    continuation-dumps.md (example without comments, its notes as prose, `--explain`
    output), cli.md (`resume --explain`, `.bt` carets, `.restore` detection); D2640,
    D2641. Guide: no page affected.

## Result
Dumps have no comments. Format 2's marker is the directive `"djs-dump 2";` (or
`"djs-dump 2 typescript";`) as the first line (D2640); version 1 (`// djs-dump 1`) still
loads; format 2 dumps with the old comment header are "not a djs dump". For humans:
`djs resume --explain dump.js` prints the open calls, the activations waiting on each and
the other suspended ones, with `dump.js:LINE:COL`, the source line and a caret, and the
bindings (D2641); `.bt` in the REPL shows the same caret lines.
Known gaps: `--explain` loads (runs) the dump, as `resume` does; activations that wait on
another activation (not a host call) are listed under "not waiting on a call".
