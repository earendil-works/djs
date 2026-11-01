# 0078: Leaf quick wins from 0077 (wasm ASCII check, UTF-8 in transcode, short_decimal early exit)

status: done
agent: num
branch: issue/0078-leaf-quick-wins
depends-on: 0077
touches: crates/djs-core/src/wire/, crates/djs-core/src/heap/ (string construction, atom hashing), crates/djs-core/src/num/format.rs, crates/djs-bench/

## Goal
deps' 0077 recommendations that need no dependency (work/issues/0077-deser-deps-investigation.md):
1. A u64-block ASCII check replacing `<[u8]>::is_ascii` in the hot paths (wire ASCII strings,
   `alloc_str`, atom hashing, heap/string.rs ~797): 10x faster in wasm on long input.
2. UTF-8 validation inside wire's `transcode`, dropping the separate `from_utf8` pass (~40% of
   wasm-side decode for large non-ASCII strings).
3. `short_decimal` gives up early on full-precision floats (it costs 12–29 ns per failure,
   20–30% of `to_string` for them).
Optional: port zmij's core for full-precision floats (≈2.5–3x native, 35–45% wasm), only if
cheap; otherwise a follow-up.

## Acceptance criteria
- [x] Each change measured native and wasm (interleaved), no behavior change (the num and
      wire corpora, fuzz tests), size noted.

## Log
- Bench: work/scratch/0078/bench (one binary, native + wasm32-wasip1 under node, built
  against a base worktree and this branch, run alternately twice; load 35–50, best of 15)
  and work/scratch/0078/setglobal.ts (djs-wasm Sandbox, base vs new interleaved, best of 7).
- D1780 block ASCII check: wasm 1 KiB 117 → 27 ns, 1 MiB 115 → 23 µs; natively slower than
  std (SIMD) for ≥1 KiB, so wasm32 only. wire decode of 1 MiB ASCII in wasip1 151 → 55 µs.
- D1781 one-pass validating transcode: wasm 1 MB mixed 1.7–1.95 → 1.04–1.09 ms, Latin-1
  2.8–3.2 → 1.9 ms; native −10–35%. Differential test against from_utf8.
- D1782 short_decimal early exit: wasm Math.random to_string 157–174 → 120–124 ns, random
  bits 180–194 → 146 ns; short decimals unchanged; native −5%. Test against the old loop.
- djs-wasm: setGlobal 4 MiB ASCII 1.74 → 1.35 ms, mixed 3.53 → 2.70, Latin-1 3.65 → 2.62;
  stringify 3×100k floats 276 → 251 ms. Size +118 B (+34 gzip).

## Follow-ups
- zmij core port in place of the Ryu port (0077: non-short floats ~35–45% in wasm): not done.
- transcode's non-ASCII path is still ~0.5–0.9 GB/s in wasm (a byte-at-a-time decode with a
  per-unit closure); a 2-byte-sequence fast path for Latin-1/Cyrillic text could double it.

## Result
Three leaf speedups, no behavior change (D1780–D1782): a block ASCII check in wasm32 (wire,
alloc_str, atom hashing, str_eq_str), UTF-8 validation folded into wire's transcode, and an
early exit in short_decimal for full-precision numbers. wasm: ASCII checks ~4x, non-ASCII
wire strings −35–40%, full-precision number formatting −20–30%; +118 B wasm.
