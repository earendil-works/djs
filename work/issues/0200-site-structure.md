# 0200: Compiler: record each site's statement path, block declarations and expression constructs with spans

status: done
agent: pjc
branch: issue/0200-site-structure
depends-on: -
guide: -
touches: crates/djs-core/src/compiler/ (recording next to the suspension maps; no behaviour change), tests/smap pins (extended), a written interface for the exporter (docs/plain-js-dumps.md or a doc comment)

## Goal
docs/plain-js-dumps.md section 8 "Recommendation and plan", step 1.

## Context
docs/plain-js-dumps.md (the design, results and plan: read all of it), the prototype in
work/research/0199/ (plain.mjs, normalize.mjs, freevars.mjs, the harness and chain checks),
decisions D2830-D2833, and today's format 2 exporter (vm/dump*.rs, D2610-D2628, D2730-D2735,
D2800-D2802). Acceptance at every merge: dumps run as plain Scripts in djs **and node** and
continue identically to the uninterrupted run over every corpus idle point they cover,
re-dumps stay bounded (cap chains at 50 links), and the existing gate checks stay green.
djs is unreleased (D024): no compatibility code for format 2 once it is retired.

## Log

- Interface first (3bac0315, merged 8b804c62): compiler/smap_syntax.rs, `FunctionMap::syntax`,
  `Site::path`/`Site::gap`, D2850. pjx's review: member targets of assignments/updates as
  `Object Property` parts; doc says where each kind of declaration is listed.
- Recording: `SynRec` in the suspension-map recorder, parser hooks `syn_open`,
  `syn_open_late` (left-first constructs, inserted into the paths of the sites since the
  left operand's mark), `syn_part`, `syn_close`, `syn_scope`, `syn_decl`, `syn_special`,
  `syn_target`; no-ops without the `dump` feature, inactive outside async
  functions/generators. Declarations go to the innermost node of their scope with a part
  containing them (D2851).
- Pins: tests/smap now include nodes and paths; `path_error` checks every corpus path;
  summary counts complete paths (1,062/1,063; one gap in a destructuring default, by
  design until 0203). Recording still changes no code (the pin test's disassembly
  comparison). New unit tests `site_paths` (a table over every node kind) and
  `site_syntax_text` (spans, decls, special refs).
- Guide: no page affected (nothing in docs/guide describes the map's fields).

## Follow-ups

- 0203: destructuring patterns as nodes (sites in pattern defaults are gaps now); class
  bodies stay unsupported. TypeScript expressions (`x as T`, `x!`, `<T>x`) are no nodes
  either: a site under one is a gap (none in the corpus).
- Constructor parameter lists (class.rs) and `Function`/`AsyncFunction` constructor
  parameters (dynamic.rs) record no `Params` part.

## Result

Every suspension map now carries the syntax around its sites (D2850, D2851):
`FunctionMap::syntax` (nodes with kind, span, parts with roles and spans, scope
declarations; `special` refs to `arguments`/`new.target`/`super`) and per site `path`
(node + the part holding the next step, from the Body down to the site's own construct)
and `gap` (the path crosses a construct that is not a node: refuse). Types and docs:
crates/djs-core/src/compiler/smap_syntax.rs. No behaviour change: same code with and
without recording (pinned), maps unchanged except for the new fields. Known gaps: patterns,
TS expressions, class bodies (see Follow-ups).
