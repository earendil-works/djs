# 0187: Remove the REPL's `.image` and the `djs heap` command (keep `.heap`)

status: in-progress
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

## Result
