# 0206: Site-structure records: pattern nodes, TS wrappers, constructor params

status: in-progress
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

## Result
