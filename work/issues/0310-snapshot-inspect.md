# 0310: Inspecting a snapshot: what suspended, as a tree

status: open
agent: pview2
branch: issue/0310-snapshot-inspect
depends-on: -
touches: crates/djs-core/src/vm/snapshot/ (inspect.rs new, describe), crates/djs-core/src/compiler (parameter names in debug info only), crates/djs-wasm (docs-module export), js/src/sandbox.ts (inspectSnapshot, docs module), crates/djs-cli (describe, --json), crates/djs-docs (gen:snapshot), docs/guide/snapshots.md (the describe section only), docs/decisions.md
guide: docs/guide/snapshots.md, docs/guide/cli.md, docs/guide/js-api.md

## Goal
The owner: when you use `debugger;` in the docs you should see what suspends, and the docs
should help someone figure out how snapshots work and how to use them. Today `describe`
prints section sizes and record counts. The investigation (prototype and mocks in
work/scratch/snapshot-view/ of the primary checkout, README there; its report is in the
coordinator's session) showed the format already holds everything; only local names need
the engine, by compiling the stored source again (as import does).

One Rust model, `snapshot::inspect`, with two outputs:
- **Text** (the new `describe`, so `djs describe` and the guide's `gen:snapshot` change by
  themselves): the run as a tree from its open calls: each activation that waits, with its
  source line and a caret, its locals and captures by name, the values they reach (records
  with named fields, references resolved, builtins and host values as names, depth/breadth
  limits for large runs), what it is in the middle of (its operand stack), up the await
  chain (promises, reactions, Promise.all elements) to the run's result; then "a loading
  build must agree on" (the bytecode hashes of the waiting functions, interface hashes, the
  builtin paths, host values, record types); then the bytes per section in one line.
- **JSON** (`djs describe --json`, and a docs-module export `djs_snapshot_inspect` with
  `Sandbox.inspectSnapshot(bytes)`): sections with byte ranges, types, strings, scripts (with
  their source), functions with spans, names and hashes, records (type, fields, byte range,
  held by), the roots tree and the needs list. Versioned with the snapshot format (D024).
- **Parameter names** in the compiler's debug info (outside the bytecode hash), so frames show
  `fetchPage(n = 1)`, not `arg 0`.
- Small fix: the strings table stores host names twice ("console", "debugger").

## Acceptance criteria
- [ ] The model, text and JSON, pinned for the guide's fetchPage example and a richer one (a
      Map, closures in boxes, Promise.all over two open calls); limits on large runs.
- [ ] `djs describe [--json]`; docs-module export and `inspectSnapshot`; the shipped module
      unchanged in size (docs module only).
- [ ] The guide's describe section and gen:snapshot show the tree; decisions recorded.

## Log

## Follow-ups

## Result
