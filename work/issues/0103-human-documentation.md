# 0103: Human-facing documentation: format proposal first, then the docs

status: open
agent: -
branch: issue/0103-docs
depends-on: -
touches: phase 1: work/issues/0103-human-documentation.md (the proposal), work/scratch/0103/ (prototypes). Phase 2 (after the owner's decision): a new docs site source directory, its build tooling, generated reference content.

## Goal
Documentation that helps a **human** understand how djs works: the interpreter design,
the heap and value model, the compiler, the builtin language and native builtins,
suspension and snapshots, determinism, the host interface, and **the trade-offs** behind
each (why a position-independent arena, why a copying GC, why no AST, why the leaf rule,
why fuel, why NaN-boxing …). Today's docs (ARCH.md, docs/*.md, the ~270-entry decision
log) are written for agents working on the code; they are accurate (0095) but not a
guided explanation.

**Phase 1 (this issue's first deliverable): decide the format.** It must stay
maintainable (lives in the repo, plain-text sources, easy to keep in sync with the code,
buildable in the gate or at least checkable) and be nice to look at (good typography,
navigation, diagrams, code samples). Building our own tool is allowed if it pays off.

Questions for the proposal:
- Source format: Markdown (CommonMark + extensions), MyST, AsciiDoc, djot, or our own.
- Generator: an existing one (mdBook, Zola, a Rust-native option; check what's installed;
  don't install anything without asking), or a small custom Rust tool in the workspace.
- Diagrams: memory layouts, value encoding, frame layout, the GC, the merge of the
  pipeline. Options: SVG generated from data (the layout tables already exist: LAYOUTS,
  the ops! table), mermaid/d2/graphviz, or hand-written SVG.
- **Generated reference sections** that can't go stale: the opcode table from ops!, the
  cell layouts from LAYOUTS / layout_json, the intrinsic list, the decision index, test262
  and perf numbers from the score files.
- Live examples: djs runs in wasm, so pages could embed a djs-wasm sandbox (run snippets,
  show bytecode via the disassembler, show heap stats, suspend/snapshot/resume in the
  browser). Worth it, and at what cost?
- Structure: a guided tour (concepts → architecture → subsystems → trade-offs → host
  interface → contributing) plus reference sections; how it relates to ARCH.md and the
  decision log (link to, don't duplicate).
- Checking: links, code samples that are executed in the gate (doc-tests), and generated
  sections verified fresh.
- Publishing: static output (GitHub Pages from the repo's `origin`), offline-readable.

## Deliverables
- [ ] Phase 1: 2–4 concrete format/tooling options with a small prototype page each
      (the same content, e.g. "Values and NaN-boxing" with a generated layout diagram),
      the trade-offs, and a recommendation. Send it to the coordinator; **don't build the
      docs before the owner has chosen**.
- [ ] Phase 2 (after the decision): the tooling, then the documentation itself in parts.

## Log

## Proposal

## Result
