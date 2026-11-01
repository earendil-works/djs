# 0077: Speedup crates used by ~/Development/deser: are they quick wins for djs? (brief investigation)

status: done
agent: deps
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
- [x] The list of deser's performance-relevant dependencies (read its Cargo.toml files and
      where they're used), and for each: what it speeds up, no_std/alloc support, wasm32
      support (including whether it needs `simd128`: djs's wasm build doesn't enable
      target features today; check what `+simd128` would mean for node/pi
      compatibility), size impact in wasm, license, and maintenance.
- [x] The matching djs code paths and today's numbers: number formatting (0004's Ryu port
      plus D1262's short-decimal fast path; compare with zmij), UTF-8 validation and
      transcoding (the lexer, wire decode, `parse_utf8`, string construction from UTF-8;
      compare with simdutf8), integer formatting (array index keys, `toString` of ints,
      JSON stringify of ints; compare with itoa), and anything else that maps.
- [x] Micro-benchmarks, native and wasm (scripts/build-wasm.sh, node), djs's code vs the
      crate, on realistic inputs (codemode-like JSON, tool output text).
- [x] A recommendation per crate: adopt (as a recorded dependency decision for djs-core,
      AGENTS.md "Dependencies"), port the idea, or skip, with the expected gain, the size
      cost and the risk. Keep it brief: this is a quick-win check, not a rewrite.

## Notes
- deser is read-only.
- Own worktree; the machine is shared (AGENTS.md "Testing while iterating").

## Log
- Read deser's Cargo.toml files and call sites. Performance-relevant: `zmij` (13 crates,
  float → text), `itoa` (8, int → text), `simdutf8` (9, behind a default `speedups`
  feature, UTF-8 validation of input). `smallvec`, `indexmap`, `hashbrown`, `arrayvec`
  are type-integration features (impls for those types), not speedups; `unicode-ident`
  is used for JSON5 identifiers. djs-core has no Rust maps in hot paths (heap tables live in
  the arena) and the lexer only consults Unicode tables for non-ASCII identifiers, so
  those are out.
- Scratch in `work/scratch/0077/` (gitignored): `bench/` (micro-benchmarks, one binary
  run natively and as wasm32-wasip1 under node via `run-wasi.mjs`; the `wasm` profile copies
  djs's opt-level "s"), `size/` (cdylib per function, wasm profile, stripped), `e2e-wasm.ts`,
  `ab.ts`, `ascii.ts` (djs-wasm through `js/src/sandbox.ts`). djs code is linked as-is
  (`djs_core::num::to_string`, and `num/ryu.rs` + `pow5.rs` via `#[path]`).
- Machine load was 24–28 on 18 cores the whole time: numbers are best-of-7 and still
  vary up to 2x between runs. Ratios within one run are stable; the tables below use the
  quietest run (`wasm-s.txt`, `native.txt`).
- One djs experiment (not committed): wire's `bytes.is_ascii()` replaced by a u64-block
  check, built into `djs-ascii.wasm`.

## Findings

Gains are per call, ns (lower is better). "wasm" = wasm32, opt-level s, LTO, node 25.

**Float → string (`num::to_string` vs zmij 1.0.23).** zmij prints Rust's format (`1e+16`,
`1000000000000000.0`, `1e-6`), not JS's, and exposes no digits/exponent API.

| input (10k values) | djs native | zmij raw native | djs wasm | zmij raw wasm | ryu crate wasm |
|---|---|---|---|---|---|
| prices, 2 decimals (D1262 path) | 15 | 9 | 23 | 34 | 52 |
| coords, 6 decimals | 46 | 12 | 68 | 36 | 49 |
| Math.random() | 47 | 18 | 72 | 43 | 38 |
| computed (i/7*1.1) | 39 | 8 | 60 | 28 | 45 |
| random × 10^[-20,20) | 56 | 12 | 99 | 53 | 62 |

- An adapter that re-lays zmij's text out the JS way (verified identical to djs on all
  50k inputs) eats most of the gain (wasm 46–97 ns): zmij is only a win if its core
  (`to_decimal`) is used directly with djs's `layout`. Estimated then: wasm ~35–45 % less
  per non-short float, native ~2.5–3x. The D1262 short path stays faster than zmij.
- djs's Ryu core alone is 48–66 ns wasm; `to_string` adds 15–35 ns on top, mostly the
  failed `short_decimal` attempt (12–29 ns when it fails, i.e. on every full-precision
  number).
- Size (wasm, stripped, one exported function): djs `to_string` 4.6 KB, zmij 3.2 KB, ryu
  crate 14 KB. zmij compresses its pow10 table like djs's `pow5`. Swapping the core is about
  size neutral (`pow5` stays for parsing).
- In context: `JSON.stringify` of a 100k float array costs 120–140 ns per number in
  djs-wasm, 50–70 for short decimals, 25–35 for ints. So formatting is half of the stringify
  time for full-precision floats, and zmij's core would save ~25–35 % of that.
  `String(x)` in a loop is ~1–1.5 µs per call, and `Math.abs(x)` ~450 ns, against
  ~70–100 ns for the bare loop. Call overhead, not formatting, dominates there (see
  Follow-ups).

**Int → string (djs digit loop in `num::format::u64_digits` / `heap::atom::format_u32` vs
itoa 1.0.18).**

| input | djs `to_string(i as f64)` wasm | djs loop wasm | itoa wasm | djs loop native | itoa native |
|---|---|---|---|---|---|
| 0..100 | 16 | 6.6 | 6.8 | 0.6 | 2.1 |
| 0..100000 | 17 | 8.5 | 6.6 | 1.5 | 2.8 |
| 13-digit ms timestamps | 21 | 11.6 | 5.9 | 1.9 | 2.6 |

itoa only wins in wasm and only for long numbers (u64 `/10` there is a 128-bit
multiply-high emulation). That is ≤ 6 ns of a 16–21 ns call, whose rest is `NumStr` and the
float checks. Then the heap allocation follows. Size: itoa 258 B vs 131 B.

**UTF-8 validation (`core::str::from_utf8`, used by wire decode and `compile`, vs
simdutf8 0.1.5).**

| input | core native | simdutf8 native (NEON) | core wasm | simdutf8 wasm | simdutf8 wasm +simd128 |
|---|---|---|---|---|---|
| 4 KB codemode script (ASCII) | 25 GB/s | 84 | 22 | 19 | 77 |
| 64 KB tool output (mixed) | 1.3 | 6.9 | 2.2 | 1.7 | 8.9 |
| 64 KB CJK-heavy | 1.2 | 7.2 | 1.8 | 1.9 | 8.6 |
| short non-ASCII, 8–48 B | 1.4 | 1.3 | 1.7 | 1.9 | 1.0 |

- Without `simd128`, simdutf8's wasm32 backend is literally `core::str::from_utf8` (its
  fallback), so it gains nothing in djs's current module. Below 64 bytes it always falls
  back, so short strings (most wire strings and JSON values) never gain.
- `+simd128` for djs-wasm alone: module −1.8 KB (1,261,895 → 1,260,091 B). No measurable
  speed change on json/strings/string-build/regexp-scan/calls (A/B interleaved, within
  noise). simdutf8 adds 2.4 KB. Wasm SIMD is in V8 since 9.1 (node ≥ 16.4; js/ requires
  node ≥ 20), Chrome 91, Firefox 89, Safari 16.4, wasmtime/wasmer by default. Pi runs on
  node, so it is safe there. Embedders on interpreters without SIMD (e.g. wasm3) would
  break. Fixed-width simd128 is deterministic (unlike relaxed-simd), so the snapshot
  and replay story (G4/0072) is unaffected.
- Where it would show: host → VM strings. `setGlobal` of a 256K-char mixed string takes
  ~470 µs (117 µs of that is the JS-side `encode`). In wasm, `from_utf8` takes about
  150–200 µs of the remaining ~350, before `transcode` decodes every byte a second time.

**Side finding: `<[u8]>::is_ascii` is slow on wasm32.** It runs at 3–5 GB/s, against
12–22 GB/s for `from_utf8` on the same ASCII input and 31–35 GB/s for a u64-block OR check
(32 bytes per branch, ~15 lines, no deps). It is wire's ASCII-string check
(`wire/mod.rs:373`), and it also appears in `Heap::alloc_str`, atom hashing and
`string.rs:797`. Measured in djs-wasm (patched wire only): `setGlobal` of a 4 MiB ASCII
string 1,210 → 890 µs (−30 % of the wasm side), +98 B. Short ASCII strings (8–48 B):
14 → 7 ns.

## Recommendations

| crate | djs path | gain native / wasm | size (wasm) | recommendation |
|---|---|---|---|---|
| zmij | `num::format::to_string` (Ryu core) | non-short floats: ~2.5–3x / ~35–45 % (only via its core, not its API); short decimals: none (D1262 is faster) | ±0 (3.2 KB vs djs 4.6 KB incl. layout; ryu.rs goes away) | **Skip as a dependency** (wrong output format, no decimal API, 72 `unsafe`). **Port the core** (Schubfach/yy `to_decimal`, MIT, a few hundred lines) in place of `ryu.rs` only if float-heavy output matters; check it against Ryu with the num corpus. ~1 day, low risk. Quick win now: make `short_decimal` fail fast for full-precision inputs (12–29 ns of 60–100). |
| itoa | `u64_digits`, `format_u32`, int `to_string` | none / ≤ 6 ns per call on 10+ digit ints | +130 B | **Skip.** If anything, port the 2-digit LUT (or split u64 into u32 halves) for the wasm u64 case; low value. |
| simdutf8 | wire `from_utf8`, `compile`'s `from_utf8(src)` | 3–6x on ≥ 64 B / **0 without simd128**, 4–5x with it; 0 on short strings | +2.4 KB plus the `+simd128` decision | **Skip for now.** It needs the module-wide `+simd128` target feature (a compatibility decision of its own) and only helps big non-ASCII host strings. **Port the idea instead:** validate inside wire's `transcode` (one strict pass instead of `from_utf8` + `transcode`), which saves the whole validation pass without SIMD or deps. |
| (no crate) u64-block ASCII check | wire ASCII strings, `alloc_str`, atom hash | 1x native (std is already fast) / 2x short, 10x long; −30 % wasm-side decode of big ASCII strings | +~100 B | **Do it**: the one real quick win found. ~15 lines in djs-core, no decision needed. |
| smallvec / indexmap / hashbrown / unicode-ident | none | n/a | n/a | **Skip**: type-integration features in deser. djs has no Rust maps in hot paths, and the lexer has its own tables for the rare non-ASCII identifiers. |

## Result
Report only; no djs code or Cargo.toml changed. Benchmarks, raw outputs (`native.txt`,
`wasm-s.txt`, `wasm-s-simd.txt`, `wasm-s-utf8.txt`, `size.txt`, `e2e-wasm-*.txt`,
`ab.txt`) and the patched module stay in `work/scratch/0077/`. No dependency is
worth adopting into djs-core. The quick win is a zero-dep ASCII check for wasm (and
optionally a single-pass validating transcode); a zmij-core port is a medium-sized
option for float-heavy output.

## Follow-ups
- wasm ASCII check: replace `<[u8]>::is_ascii` in wire decode, `Heap::alloc_str`, atom
  hashing and `heap/string.rs:797` with a u64-block check (wire alone: −30 % of the wasm
  side for big ASCII strings).
- Wire decode of non-ASCII strings: fold strict UTF-8 validation into `transcode` and drop
  the separate `from_utf8` pass (~40 % of the wasm side for big mixed strings).
- `num::format::short_decimal`: fail fast on full-precision numbers (it costs 12–29 ns,
  20–30 % of `to_string`, every time it fails).
- Optional: port zmij's `to_decimal` in place of the Ryu port (non-short floats ~35–45 %
  faster in wasm, size neutral).
- Not in scope, but larger than anything above: in djs-wasm `String(x)` costs ~1–1.5 µs
  per call, `x.toString()` ~700 ns and `Math.abs(x)` ~450 ns, against ~70–100 ns for the
  bare loop and ~300 ns for `"" + x` (loaded machine, so absolute numbers are inflated,
  but the ratios hold). Builtin call overhead looks worth a profile.
