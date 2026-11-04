# 0206: Site-structure records: pattern nodes, TS wrappers, constructor params

status: done
agent: pjc
branch: issue/0206-site-structure-gaps
depends-on: 0200
guide: -
touches: crates/djs-core/src/compiler/ (smap_syntax recording), tests/smap pins

## Goal
0200's follow-ups, so 0203 (pjx) can write plain continuations for sites inside them:
- pattern nodes (array/object destructuring, defaults, rest) on site paths, instead of a gap;
- TypeScript expression wrappers (`as`, `!`, `satisfies`, type assertions) as nodes or
  transparent steps, instead of gaps;
- a Params part for class constructors and Function-constructor parameters.
Interface additions documented in smap_syntax.rs; pins; no behaviour change. Agree node
shapes with pjx (via the coordinator) before building.

## Log

- Shapes agreed with pjx via the coordinator: patterns as `ArrayPattern`/`ObjectPattern`
  nodes with a `Default` node for targets with defaults (D2852); TS transparent via
  `Syntax::erased` + splicing from `strip_types` (D2853); `Decl::at` (pjx asked for names
  per declarator).
- pattern.rs: `ArrayPattern`/`ObjectPattern` nodes, parts per element/property,
  `Default` opened late after the target (inserted into the paths of sites in a reference
  target) and closed after the store, so nested patterns (parsed at the store) nest in
  it. for-in/of `Head` parts may hold a declaration keyword before the pattern node (the
  gap check allows it).
- ts.rs: `ts_erase_kind` also records the range in the current function's recorder.
- Params part in class constructors and dynamic.rs (no sites possible there today).
- Pins: 1,063 of 1,063 corpus sites complete now; tests `site_paths` (pattern cases) and
  `site_paths_through_typescript`. Recording still changes no code.
- Guide: no page affected.

## Result

Sites inside destructuring patterns and under TypeScript expressions have complete
paths: pattern nodes `ArrayPattern`/`ObjectPattern`/`Default` (D2852), TS ranges in
`Syntax::erased`, transparent to the gap check (D2853), `Decl::at`, Params parts for
constructors and Function-constructor sources. All documented in
crates/djs-core/src/compiler/smap_syntax.rs. Every corpus site (1,063) has a complete path.
