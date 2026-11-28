# 0310: Inspecting a snapshot: what suspended, as a tree

status: done
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

**Owner, on what the view shows:** much more human friendly, so a reader can interactively
understand what is in the snapshot. Lead in JavaScript terms (functions stopped where, with
their variables as JS values; what each waits for; the result still pending), explain each
part in one plain sentence (what it is and why it is in the snapshot), show identity by
highlighting the same object reached twice, and keep engine detail in layers one can open:
the JS view first, then how djs stores it (records, named fields, sizes), then bytes. Sizes
in plain words. The JSON carries per node a human label, a one-line explanation and the JS
rendering, then the record and byte details.

## Acceptance criteria
- [x] The model, text and JSON, pinned for the guide's fetchPage example and a richer one (a
      Map, closures in boxes, Promise.all over two open calls); limits on large runs.
- [x] `djs describe [--json]`; docs-module export and `inspectSnapshot`; the shipped module
      unchanged in size (docs module only). (+181 bytes, 102 gzipped: the parameter names
      and the string reuse, both in shared code; the inspector itself is not in it.)
- [x] The guide's describe section and gen:snapshot show the tree; decisions recorded.

## Log
- Read the prototype (work/scratch/snapshot-view). `Snap::parse` and its types are now
  `pub(super)` for the inspector; the old `describe` (section sizes and record counts) is
  gone.
- Owner's steer mid-way: lead in JavaScript terms, explain each part in a sentence, keep
  storage details one layer down, identity visible without ids, sizes in words. The model
  became a tree of sentence nodes (`view`) with notes and `detail` lines, rendered by the
  text and carried in the JSON; records got label, note, `js`, a structured `view` and
  `known` (where the text first showed them). Told the coordinator: moderate growth, no
  scope change.
- The bytecode hash cannot be computed in the inspector's bare heap (it names builtins
  through a realm): functions are matched by span, by interface hash when two share one.
- Size: with the inspector in djs-core unconditionally the shipped module grew 4.4 KB
  although the inspector is dead there: its BTreeMap/BTreeSet instances were shared with
  the loader and changed its inlining. Now a feature `inspect` (default; djs-cli, djs-wasm
  `docs`), not in the build hash, and lists instead of maps. Measured with
  scripts/wasm-funcs.py against a worktree of main.
- Parameter names: appended after the captures in `FuncInfo::NAMES` for simple
  parameters (the cheapest of three forms measured: a loop with push).
- Tests: inspect_tests.rs (pinned text for fetchPage and the rich example, limits, no
  sources, activations no call leads to, every idle point of tests/snapshot/programs as
  text and valid JSON); js snapshot and sleep-snapshot tests on the JSON; CLI test of
  `describe --json`.
- Guide pages updated: snapshots.md (the paragraphs after the example's snapshot, the
  describing section, where to look), cli.md, js-api.md, host.md (the docs ops), and the
  regenerated gen:snapshot regions of snapshots.md and suspension.md. Also
  examples/sleep-snapshot/README.md (its describe output and the walk through it).

## Follow-ups
- 0311 builds the docs page on the JSON (`view`, records' `view`/`known`/`note`, `sizes`).
- `doing` reads the operand stack heuristically (innermost callee and `this`, an
  ArrayIterator); other constructs (spread, template literals, for-in state) fall back to
  "halfway through an expression, holding …".
- Duplicate simple parameter names (`function f(a, a)`, sloppy) name the arguments in
  declaration order of their bindings, which may be off by one there.

## Result
`snapshot::inspect(bytes) -> Inspection` (feature `inspect`), `Inspection::text()` (=
`describe`) and `Inspection::json()`. `djs describe [--json] FILE`; docs module
`djs_snapshot_inspect`, `Sandbox.inspectSnapshot(bytes): SnapshotInspection` (ABI 15).
The text leads with the run in JavaScript terms (open calls, the activations waiting on
them with source lines, variables as literals, what they are halfway through, up to the
run's result), then what a loading djs must have, the size in words, and the records as
stored. Frames name their parameters (compiler debug info). Host names are stored once in
the strings table. D3760.
