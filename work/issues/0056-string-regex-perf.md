# 0056: String and regex hot-path performance (+ the test262 property-escape tests' gate cost)

status: done
agent: num
branch: issue/0056-string-regex-perf
depends-on: 0050
touches: crates/djs-core/src/regexp/, crates/djs-core/src/builtins/ (string/regexp leaves), builtins/ (string.ts, regexp.ts, function.ts apply), crates/djs-core/src/heap/ (string building helpers, additive), crates/djs-core/src/interp/ (only isolated fast paths, coordinated), crates/djs-bench/ (benchmarks), crates/djs-core/src/json/ (only for the optional JSON source-text item)

## Goal
After 0050's promotion, the active test262 run costs ~250 CPU-s more (~30 s wall at
-j 18) because the harness's `buildString` (regExpUtils.js) builds 1M-code-point strings:
a JS loop `codePoints[length++] = cp` plus `String.fromCodePoint.apply(null, chunk)` with
10k arguments, then `+=`, and the property-escape tests match regexes over them. That
cost hits every gate. The same kinds of hot paths matter for codemode (string building,
regex over large tool output).

## Acceptance criteria
- [ ] Profile the slowest test262 property-escape tests natively and record where the
      time goes (the element-store loop, `apply` with a big argument array,
      `fromCodePoint`, rope concatenation/flattening, the regex scan with \p tables, exec
      copying the program out of the heap: 0050 follow-up, 11–136 KB per exec).
- [ ] Fixes for what dominates, for example: `Function.prototype.apply` with a dense
      array (no per-argument slow path), a bulk `fromCodePoint`/`fromCharCode` leaf
      that writes straight into a flat string, exec reading the program in place from
      the heap (zero-copy, or a per-RegExp cached decode), rope-friendly `+=`, and
      regex literal evaluation cost (0050: 8x slower than qjs).
- [ ] Before/after: the gate's test262 step (CPU-s and wall), the slowest 20 tests, and
      djs-bench numbers (add string-building and regex-scan benchmarks).
- [ ] No semantic changes: ratchets unchanged or better, stress modes identical.
- [ ] Optional, if time permits: JSON.parse source text access (0050 follow-up:
      `{source}` in the reviver, `JSON.rawJSON`, `JSON.isRawJSON`, stringify support).

## Notes
- heap (0051) is in builtins/ and realm; infra (0054) in builtins/global functions;
  devflow (0053) changes the test runners. Keep interp changes isolated and tell the
  coordinator before touching shared hot paths in run.rs.

## Log
- **Profiling setup.** `sample` on a release djs running harness + test files,
  repeated in blocks (work/scratch/0056: prof.sh, incl.py for inclusive counts,
  ab.sh for interleaved min-of-N A/B against a release build of main b8eefec).
- **Before.** General_Category_-_Decimal_Number.js took 0.39 s of CPU:
  - interpreter 56% (buildString's element loop, plus the prelude loops in
    CreateListFromArrayLike for `apply` and in String.fromCodePoint, both
    per-element in the builtin language);
  - regex matching 18% (STAR's per-character `single()`);
  - the rest allocation and GC.

  Regex literal `exec` in a loop:
  - leaf 48%: matcher 19%, match array 18%, atom_str lookups 6%;
  - GC 12%, Rust reallocations 8%, builtin-language overhead about 35%.
- **D1560** (exec/test hot path). 2M literal evaluations: `exec` 1.91 -> 1.33 s,
  `test` 2.33 -> 1.12 s.
- **D1561** (apply with dense arrays, `%AllCodePoints`/`%AllNumbers`). Decimal_Number
  test 630 -> 270 ms.
- **D1562** (tight STAR scan with a range hint). 270 -> 140 ms.
- **D1563** (follow sets accepting `$`; F_DISJOINT). pi's base64 check failing on 6 MB:
  1120 -> 220 ms (3 runs). Decimal_Number -> 110 ms.
- **Well-known atoms** for the RegExp protocol names (heap/atom.rs, additive;
  js/src/layout.generated.ts regenerated), and `%WellKnownSymbol` without a String
  allocation (interp/run/intrinsics.rs, one isolated arm).
- **Benchmarks.** New engine scripts tests/bench/string-build.js and regexp-scan.js. New
  core benchmarks: regexp/scan `\p{L}+` (multilingual) and the base64 check.

  scripts/bench.sh, before -> after (medians):

  | benchmark | before | after |
  | --- | ---: | ---: |
  | regexp/scan email | 50 MB/s | 70 MB/s |
  | `\d+` | 102 MB/s | 165 MB/s |
  | `\s+` | 34 MB/s | 52 MB/s |
  | `\p{L}+` | 33 MB/s | 69 MB/s |
  | base64 check | 19 MB/s | 25 MB/s (before D1563's disjoint flag) |
  | engine/djs/string-build | 122 ms (24x qjs) | 49 ms (4.4x) |
  | engine/djs/regexp-scan | 148 ms (9.1x) | 59 ms (2.5x) |

  engine/djs/regexp went 316 -> 365 ms median (best 303 -> 163). The medians of that
  noisy n=5 run disagree.
- **test262 active, release, -j 6** (two runs each): CPU 250-257 -> 90-91 s; wall
  61-66 -> 17-18 s.
  - Slowest after: the three TypedArray copyWithin detached tests (1.5-2.3 s each, not
    regex); character-class-escape-non-whitespace.js 0.49 s (65k replace calls,
    interpreter-bound); the property-escape tests at 0.2 s each.
  - Ratchets unchanged; difftest with --suspend-every 97 --gc-stress OK.
- crates/djs-bgen/tests/behavior.js: stand-ins for IsDenseArray, AllNumbers and
  AllCodePoints (outside touches, test only).

## Follow-ups
- What remains in the property-escape tests and character-class tests is the
  interpreter: the harness element loop and builtin-language glue (indexOf, assert,
  replace). That is interpreter speed, not this issue.
- `s += x` in a loop builds rope chains that every GC traces. A flat string with spare
  capacity that appends in place when the left operand is its current end (heap string
  layout) would fix it: 2M appends take 320 ms, qjs 80 ms.
- Exec still copies the program out of its cell, now in one bulk read: about 1 µs for
  `\p{L}`, 15 µs for `\p{RGI_Emoji}`. A cache in Exec keyed by cell and GC epoch, or a
  matcher that reads words in place, would remove the copy.
- The TypedArray copyWithin detached tests (1.5-2.3 s each) are the slowest in the
  active group: worth a look by their owner.
- JSON.parse source text access (optional item): not done; see 0050's follow-up
  estimate.

## Result
- The test262 active run costs 90 CPU-s instead of 250 (wall 17 s instead of 61 s at
  -j 6).
- Regex `exec`/`test` in loops are 1.4-2x faster, and scans with long single-class runs
  1.5-2x faster.
- `apply`, `fromCodePoint` and `Math.max` with big argument lists take bulk paths.
- pi's base64 check fails in one pass.
- No semantic changes: ratchets unchanged, stress modes identical.
- JSON source text access was not done (optional).
