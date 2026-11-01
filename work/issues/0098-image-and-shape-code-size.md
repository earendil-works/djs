# 0098: Base image and shapes code size (0083 part 4 + D016 follow-ups)

status: open
agent: -
branch: issue/0098-image-and-shape-code-size
depends-on: 0083
touches: crates/djs-core/src/heap/ (compact_for_image, shapes, the atom index), crates/djs-core/src/bytecode/template.rs (slim privileged templates), crates/djs-prelude/, crates/djs-core/src/interp/run/ic.rs (code size only)

## Goal
0083's leftovers plus wasm-first size (D016): shapes added 12.2 KB of wasm code (data −8.6
KB), and part 4 was left optional: slimmer privileged (prelude) function templates
(~80 → 48 B each), the atom index load factor, and compact_for_image shrinking descriptor
and transition arrays (~1 KB). The base image is 398 KB of the 1.5 MB module.

## Acceptance criteria
- [ ] Part 4 items, each measured (base image bytes; wasm data and code; gzip).
- [ ] The shapes/IC code size reviewed with scripts/wasm-funcs.py: out-of-line cold paths,
      fewer monomorphized copies; no speed loss (wasm instructions, engine track).
- [ ] Determinism checks and image cross tests unchanged in behavior; images may change
      (build hash).

## Log

## Follow-ups

## Result
