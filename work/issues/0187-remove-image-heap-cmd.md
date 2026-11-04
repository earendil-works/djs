# 0187: Remove the REPL's `.image` and the `djs heap` command (keep `.heap`)

status: done
agent: snapshot
branch: issue/0187-remove-heap-cmd
depends-on: 0177
guide: docs/guide/snapshots.md (it shows a `djs heap` diff), any page mentioning `djs heap` or `.image`
touches: crates/djs-cli/src/main.rs, crates/djs-cli/src/repl/, crates/djs-cli/src/heap/ (keep what `.heap` needs; drop file reading of images and anything only the command used), crates/djs-cli/tests/, scripts/heap/ and other scripts that call `djs heap`, docs/cli.md, docs/guide/, README.md, TESTING.md/ARCH.md mentions

## Owner
"Remove `.image` and the `heap` command (but not `.heap`). We no longer need that."

## Goal
- Remove the REPL command `.image FILE` and the `djs heap <dump>` subcommand.
- Keep the REPL's `.heap [sections]` (the analyzer over the live VM against the base image),
  and the analyzer code it uses; delete whatever only the command or image files needed
  (raw image file reading, --reference handling, JSON-to-file paths if unused), and their
  tests; keep `.heap`'s tests.
- Update docs (cli.md, the guide's snapshots page and any other mention, README), scripts
  that call `djs heap`, and the decision log (a short decision noting the removal).
- Note: example (0186) is editing ARCH.md/TESTING.md prose right now; if they mention
  `djs heap`, tell the coordinator instead of editing them.

## Log

- Removed `djs heap` (main.rs dispatch and usage; heap/mod.rs `main`, `run`, `to_json`,
  file reading, `--diff/--only/--all/--json/--top` handling) and the REPL's `.image`
  (command, help line, `write_image`). `analyze_image` is now `heap::analyze(image,
  reference, opts)`; `.heap` calls it as before.
- What only image files needed: the summary's `file:` line and the dependency-descriptor
  line (and its `deps`/`runs_here`/`file*` JSON fields, `dep_key`), `Options::all` (the
  `--all` whole-image view; without a reference the analyzer still covers the whole
  image), the `(--all ...)` hint. `Section` keeps `key`/`json`: the heap tests check the
  JSON form, nothing prints it (allowed as dead outside tests, with a comment). json.rs
  stays (tools.rs uses it).
- Tests: dropped `an_image_file_reads_like_the_image` (its prefix assertion moved to the
  sandbox test); the session-part test compares with `analyze(.., None, ..)` instead of
  `--all`; repl.rs: `.image` test became `restore_reads_only_dumps` (a non-dump file),
  `.help` no longer lists `.image`, and `.image x` is an unknown command.
- Docs: docs/cli.md (usage, "The heap report (`.heap`)" replaces "`djs heap`", REPL
  table), guide snapshots.md ("Inspecting a heap" is about `.heap`; the stress-tests line;
  where to look), docs/host.md, README (repl line), registry.rs doc comment, research
  notes (scripts/snapshot-research/README.md, empty-heap.md and snapshot-versioning.md
  reproduction commands marked as needing an older checkout). ARCH.md and TESTING.md had
  no `djs heap` mention. Other research reports mention `djs heap` historically; left.
- D2269.

## Result

`djs heap` and the REPL's `.image` are gone; `.heap [sections]` keeps the analyzer
(summary, kinds, prefix, diff, largest, strings, shapes, functions, session) over the live
VM against the base image. D2269.
