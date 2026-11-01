# 0085: Leaf fast paths for everyday builtins (0081 proposal 1)

status: done
agent: perf
branch: issue/0085-everyday-builtin-fast-paths
depends-on: 0081
touches: builtins/ (string.ts, math.ts, number.ts, object.ts: the fast-path dispatch only), crates/djs-core/src/builtins/ (new leaf intrinsics), crates/djs-core/src/bytecode/intrinsics.rs (regenerated), crates/djs-bench/ + scripts/perf/ (measurements)

## Goal
0081 F1: in the pi-like script 95% of executed bytecodes are prelude builtins, and
everyday builtins are 4–17x slower per call than quickjs-wasi (`Math.max` 17x, `padEnd` 14x,
`String(i)` 12.5x, `includes` 7.4x, `split` 6.7x, `hasOwnProperty` 6.6x). Give them a
leaf fast path in Rust when the receiver and arguments are primitives (or plain objects for
hasOwnProperty), falling back to the spec path otherwise, so no user code can observe the
difference (the D1351 rule: check whatever the spec would look up).

## Acceptance criteria
- [x] Fast paths for the hot set measured by 0081 (String: includes, indexOf, split with a
      string separator, slice, substring, trim*, pad*, startsWith/endsWith, repeat,
      toLowerCase/UpperCase, replace/replaceAll with string patterns; `String(x)`/
      ToString of numbers; Math.max/min/floor/ceil/round/abs; Number.prototype.toString
      /toFixed; Object.prototype.hasOwnProperty / Object.hasOwn; Array.isArray, …),
      chosen by instruction counts on the pi-like script and wmicro.py.
- [x] Coordinate with infra (0080: for-of, native builtins quick fixes, natives wave 2
      including Object.keys/entries and JSON) via the coordinator: this issue owns
      String/Math/Number and hasOwnProperty/hasOwn leaf fast paths.
- [x] Per-call instruction counts before/after (wmicro.py, wasm primary per D016), and the
      pi-like script; difftest/test262 identical, both backends.

## Log

- Design (D1850): a `@fast` tag in the TS source, a Rust entry of the same name in
  `builtins/fast.rs`, `%MarkFast(f)` from the prelude (`FuncTemplate::FAST` in the cell's
  padding). With the native backend, a call first runs the entry; it returns the result
  or declines. Hooked in `Exec::call` (natives' `n_call`, intrinsics, `%Call`) and in the
  run loop after `fast_step` declines a `call` instruction. First tried inside
  `fast_call_at` (one `step` less per call), but that perturbed `fast_step`'s code
  (D1352); outside it, plain loops stay within ±1.5% (wasm, n=3: tight +0.2%, loop
  +0.7%, call1 0%, calls +1.1%, props +1.5%) for ~150 more instructions per fast call.
  Outside `touches:` (minimal): crates/djs-bgen (the tag), heap/cells.rs (the field),
  bytecode/template.rs (the flag), interp/run.rs + interp/run/fastpath.rs (the hook),
  js/src/layout.generated.ts (regenerated).
- The conversion helpers every bytecode builtin calls (ToString, ToNumber,
  ToIntegerOrInfinity, ...) got entries too: pi-like's prelude bytecodes went from 64.1M
  to 1.1M, bytecode frames from 4.9M to 0.56M (instr.py).
- `builtins/string.rs`: `join` writes a string from pieces once (short ones gathered on
  the stack), substring and repeat use it, and `index_of` keeps a short needle on the
  stack and compares bytes for one-byte strings: no Rust heap allocation per call.
- ArraySpeciesCreate (coordinator: in scope): plain Array, no own `constructor`,
  prototype %Array.prototype% whose `constructor` is the data property %Array%, whose
  @@species is the original getter (ArrayConstructor and ArraySpecies became
  `@intrinsic` so their identity can be checked); else declines.
- Found on the way: an entry's fuel per receiver code unit made `test_rope.js` run out
  of fuel; now 2 fuel per fast call, like the string leaves. A host allocation refusal in
  the fast call from the run loop was retried by `step` and so swallowed
  (`host_memory_failure_is_not_a_js_error`); now it stops the run (D1690).
- Tests: tests/corpus/builtins/fast-paths.js (every entry, its declines: objects with
  valueOf/toString, symbols, bigints, proxies, patched prototypes and species, RangeErrors,
  `$` patterns), both backends and the stress modes identical; test262 built-ins/{Array,
  String, Math, Number, Object, parseInt, parseFloat, isNaN, isFinite} no new failures;
  difftest ratchet unchanged on both backends; unit test that every entry is tagged.
- Measurements (`scripts/perf`, DJS_PERF_DIR=work/scratch/0085; base = main 9fb34a3):

  wasm (node 25, pi's 10k fuel slices), instructions of whole workloads, n=3:

  | workload | before | after | quickjs-wasi | ratio after |
  | --- | ---: | ---: | ---: | ---: |
  | pi-like | 25,829M | 12,924M (−50%) | 7,260M | 1.78 (was 3.56) |
  | strings | 8,644M | 6,335M (−27%) | 6,004M | 1.06 |
  | tight / loop / call1 / prop1 | 14,173 / 13,339 / 7,811 / 4,966M | +0.2 / +0.7 / 0.0 / +0.1% | | |

  pi-like cycles 4,236M → 2,308M (−46%). Native: pi-like 13,848M → 5,901M instructions
  (−57%, 4.1x → 1.8x qjs), strings −30%, plain loops unchanged.

  wasm per call (wmicro.py, machine instructions, single runs, ±10% on small ops):

  | call | before | after | quickjs-wasi |
  | --- | ---: | ---: | ---: |
  | `line.includes('throw')` | 23,550 | 3,154 | 3,193 |
  | `'a,b,c'.split(',')` | 41,970 | 9,995 | 6,206 |
  | `s.padEnd(20)` / `padStart(8, '0')` | 40,334 / 36,480 | 4,599 / 3,672 | 2,950 / 2,680 |
  | `String(i)` / `String(s)` | 21,541 / 20,372 | 2,875 / 1,625 | 1,699 / 1,180 |
  | `Number(s)` / `parseInt(s, 10)` | 23,871 / 8,610 | 2,502 / 2,449 | 2,370 / 2,414 |
  | `Math.max(i, 3)` / `Math.floor(x)` | 25,828 / 5,225 | 1,951 / 2,019 | 1,482 / 1,545 |
  | `s.startsWith` / `endsWith` / `lastIndexOf` | 10,224 / 20,990 / 20,183 | 2,694 / 2,521 / 2,817 | 1,815 / 1,791 / 1,953 |
  | `s.indexOf('o')` / `slice` / `substring` | 7,980 / 9,755 / 9,828 | 2,811 / 3,550 / 3,494 | 1,868 / 2,102 / 2,098 |
  | `s.trim()` / `toUpperCase()` / `repeat(3)` | 9,841 / 10,153 / 21,115 | 3,500 / 3,578 / 4,634 | 1,908 / 2,380 / 2,283 |
  | `s.replace` / `replaceAll` (strings) | 40,100 / 71,567 | 6,042 / 9,366 | 4,315 / 5,989 |
  | `charAt` / `at` | 7,193 / 7,825 | 3,255 / 3,513 | 1,813 / 1,838 |
  | `(i).toString()` / `toFixed(2)` | 8,644 / 12,624 | 2,506 / 4,225 | 1,778 / 2,690 |
  | `hasOwnProperty` / `Object.hasOwn` | 11,159 / 10,794 | 3,567 / 3,668 | 1,704 / 1,926 |
  | `Array.isArray` | 2,627 | 1,846 | 1,346 |
  | `a.map(x => x)` (len 4, ArraySpeciesCreate) | 69,806 | 26,468 | 12,247 |

  Code size: wasm code section +24 KB (832 → 857 KB, +2.9%), data +2 KB.

## Follow-ups

- The remaining per-call gap (1.2–2x quickjs-wasi) is the call path itself: `get_method`
  + the slow `call` (~1,000 wasm instructions before the entry runs). A cheaper hand-off
  needs the run-loop work of 0081 P6 or a fast-path test inside `fast_step` once its
  register allocation is less fragile.
- infra (0080) moves its dense-array leaves (push/pop/indexOf/slice/join) to `@fast`.
- pi-like's rest: user bytecode (3.3M), natives map/filter/slice/join/ToStringPrimitive
  (0080), `ArrayFrom` (1,947 bytecodes per call) and JSON/RegExp wrappers.
- `@fast` candidates by the same rule: `Object.keys`/`entries` of plain objects (infra's
  natives wave), `String.fromCharCode`, `Math.pow`/`atan2`/`hypot`, `Number.isInteger`.

## Result

Landed: `@fast` leaf fast paths (D1850). A builtin tagged `@fast` has a Rust entry in
`crates/djs-core/src/builtins/fast.rs` that runs in place of the call (native backend)
when its arguments make the result unobservable, and declines otherwise. 57 entries:
the conversion helpers of the builtin language, String, Number, Math, parseInt/parseFloat/
isNaN/isFinite, hasOwnProperty, Object.hasOwn, Array.isArray, ArraySpeciesCreate. pi-like
−50% wasm instructions (1.8x quickjs-wasi, was 3.6x), −57% native; everyday calls 3–15x
cheaper, `includes`/`Number()`/`parseInt` at quickjs-wasi parity. Plain bytecode
unchanged (±1.5%). +24 KB wasm code. How to add one: tag the TS function `@fast`, add
`f("Name", fn)` to `FAST` mirroring the TS step by step, decline on everything else, add
cases to tests/corpus/builtins/fast-paths.js. Known gaps: see Follow-ups.
