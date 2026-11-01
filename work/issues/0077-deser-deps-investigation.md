# 0077: Speedup crates used by ~/Development/deser: are they quick wins for djs? (brief investigation)

status: open
agent: -
branch: issue/0077-deps-investigation
depends-on: -
touches: work/issues/0077-deser-deps-investigation.md (the report), work/scratch/0077/ (benchmarks and experiments). No changes to djs code or Cargo.toml on main.

## Goal
The owner points out that `~/Development/deser` (read-only) uses small crates that speed
up common operations, which may work with wasm and give djs quick wins. A quick look at its
dependency list shows, among others, `zmij` (float → string), `simdutf8` (UTF-8
validation), `itoa` (integer → string), plus `smallvec` and `indexmap`. Investigate
briefly, with numbers.

## What to deliver
- [ ] The list of deser's performance-relevant dependencies (read its Cargo.toml files and
      where they're used), and for each: what it speeds up, no_std/alloc support, wasm32
      support (including whether it needs `simd128`: djs's wasm build doesn't enable
      target features today; check what `+simd128` would mean for node/pi
      compatibility), size impact in wasm, license, and maintenance.
- [ ] The matching djs code paths and today's numbers: number formatting (0004's Ryu port
      plus D1262's short-decimal fast path; compare with zmij), UTF-8 validation and
      transcoding (the lexer, wire decode, `parse_utf8`, string construction from UTF-8;
      compare with simdutf8), integer formatting (array index keys, `toString` of ints,
      JSON stringify of ints; compare with itoa), and anything else that maps.
- [ ] Micro-benchmarks, native and wasm (scripts/build-wasm.sh, node), djs's code vs the
      crate, on realistic inputs (codemode-like JSON, tool output text).
- [ ] A recommendation per crate: adopt (as a recorded dependency decision for djs-core,
      AGENTS.md "Dependencies"), port the idea, or skip, with the expected gain, the size
      cost and the risk. Keep it brief: this is a quick-win check, not a rewrite.

## Notes
- deser is read-only.
- Own worktree; the machine is shared (AGENTS.md "Testing while iterating").

## Log

## Findings

## Recommendations

## Result
