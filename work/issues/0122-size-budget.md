# 0122: wasm size toward the D020 budget (gzip <= 1.5x quickjs-wasi)

status: open
agent: -
branch: issue/0122-size-budget
depends-on: 0098, 0116
guide: docs/guide/goals.md and any page quoting sizes (generated numbers regenerate)
touches: crates/djs-core/ (broadly: generic code, tables, builtins), crates/djs-bgen/ (generated code size), crates/djs-wasm/, scripts/build-wasm.sh, Cargo profile settings

## Goal
The owner accepted D020: the shipped module's gzip size must stay within 1.5x
quickjs-wasi's. Today ~528 KB vs ~287 KB (~1.8x); the target is <= ~430 KB gzip, ~100 KB
to cut. Earlier size work: 0098 (base image + shapes), the bloat agent's
analysis (twiggy/attribution scripts), wasm-opt rejected (hurts gzip).

## Acceptance criteria
- [ ] An up-to-date size attribution (code by crate/module/function family, data: base
      image, tables, strings) and a ranked list of candidate cuts with estimated gzip wins
      and instruction costs.
- [ ] Land the cuts that pay, with no workload slower than +1% in wasm instructions and
      pi-like unchanged or better, unless a trade is recorded as a decision. Candidates to
      examine: monomorphization in generic code, bgen-generated code (fast paths that
      don't pay), panics/formatting machinery, duplicate Unicode/regex tables, base image
      content (lazy-materialized builtins?), opt-level per crate/function.
- [ ] The size track enforces the budget (gate fails when gzip > 1.5x the recorded
      quickjs-wasi size, or a ratchet toward it), documented in TESTING.md.
- [ ] Report where we end up; if 1.5x isn't reachable without real cost, say what it
      would take.

## Log

## Follow-ups

## Result
