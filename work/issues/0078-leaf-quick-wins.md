# 0078: Leaf quick wins from 0077 (wasm ASCII check, UTF-8 in transcode, short_decimal early exit)

status: open
agent: -
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
- [ ] Each change measured native and wasm (interleaved), no behavior change (the num and
      wire corpora, fuzz tests), size noted.

## Log

## Follow-ups

## Result
