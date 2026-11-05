# 0233: Performance: regexp exec/replace/matchAll and compile

status: done
agent: pre
branch: issue/0233-perf-regexp
depends-on: -
guide: builtins.md (regexp) if it changes
touches: crates/djs-core/src/regexp/, builtins/regexp.ts, string replace/matchAll

## Goal
Measured by benchpage (0229) against the best QuickJS wasm build (QuickJS 2026-06-04 / QuickJS-NG 0.18, wasi-sdk -O2), node 26; scripts in .worktrees/benchpage/work/scratch/0229/ops.ts, ops2.ts (need vendor/engines from scripts/bench-engines.sh --setup on that branch).
- replace(/\s+/g, ' ') 3.0x slower than Bellard QuickJS (djs beats NG 1.6x); exec of
  /^([^:]+):(\d+):(.*)$/ per line 2.2x; matchAll with /gm 2.0x; test /\b(TODO|FIXME)\b/ 1.6x;
  new RegExp compile 4.3-4.9x (after 0224's fix of the follow-set regression).
These are the text-processing workloads (8% of codemode bodies; the "loop" workload is 1.7x
slower because of the replace). Find root causes (matcher inner loop, captures allocation, the
replace/matchAll builtins in bytecode, compile allocations) and fix; target parity with QuickJS.
Constraints: exact semantics (test262 and the difftest stay green); measure in wasm instructions on the shipped module before/after and report size cost (the shipped module has ~15 KB gzip headroom shared by 0230-0234: keep each change small, report bytes; the coordinator arbitrates); several agents work in djs-core at once (0230 builtin calls, 0231 arrays/strings/collections, 0232 async, 0233 regexp, 0234 memory): stay in your touches, ask the coordinator before changing shared paths (the interpreter's call path, the heap). Batch merges, gate checks locally, niced builds, benchmarks only at load < ~4.

## Log

Measuring: work/scratch/0233/m.py runs synthetic workloads like benchpage's ops (w/*.js from
gen.py: 4000 lines of source-like text, 200 prose paragraphs) in the perf wasm build (unsafe
heap, as shipped) and in Bellard QuickJS 2026-06-04 (vendor/engines), and reports node's
instructions as the slope between 1 and 4 repetitions (V8's tier-up and setup cancel out;
single counts carried 300-500 M of fixed cost). Minstr per repetition, base = 8f2f839f:

| workload | base | QuickJS |
| --- | ---: | ---: |
| replace /\s+/g | 916 | 372 |
| exec /^([^:]+):(\d+):(.*)$/ per line | 726 | 397 |
| matchAll /^export .../gm | 386 | 248 |
| test /\b(TODO\|FIXME)\b/ per line | 390 | 371 |
| new RegExp('report\|metric'+i, 'i') | 264 | 96 |
| match /function \w+/g | 246 | 241 |

Root causes found (profiles: node --cpu-prof of the wasm module, samply natively):
- Every exec copied the program out of its cell (an allocation) and allocated slots, slot
  generations, a 64-word stack and the captures: 5 allocations per exec, per match in a
  global replace.
- The search was bytecode around the pattern (`SPLIT_GOTO; ANY; GOTO`, a `SCAN`): every
  attempt pushed a frame, so every capture write was logged and undone. A pattern starting
  with `^` (no `m`) was tried at every position of a line that failed (the exec workload: the
  lines without a colon), a multiline one at every first character of the text.
- A loop at the end of a pattern (`\s+`) left a backtrack frame per match.
- replace built UTF-16 per unit and narrowed it at the end; its output grew by doubling.
- compile: first_chars allocated a set per character and a union per element; the parser's
  term vectors grew from empty; rope patterns were read unit by unit through the chunk
  iterator; RegExpConstructor ran IsRegExp, GetPrototypeFromConstructor and ToStrings for
  string arguments.
- exec/test/matchAll's next looked up `exec` and `lastIndex` through Get (atoms interned per
  call); matchAll recompiled its regexp (Construct) and called the 8 flag getters twice.

Done so far (commits on the branch):
- D3135: the search is the matcher's (PC_FIND): attempts on an empty stack, first set with a
  Latin-1 bitmap, `^` anchors (input start: one attempt; line starts: skips to the next line
  terminator), the leading STAR's skip; programs read in place (`Prog`, byte quads of the
  cell); one `ExecState` (slots, generations and stack in one Vec) reused across the matches of
  replace and split; `F_LAST` STAR without frame; replace output one byte per unit when it can
  be, sized to the input. Fuel stays additive (tests `search_fuel_is_additive`,
  `searches_need_no_stack`, `anchored_patterns_start_where_they_can`).
- Compile: first set accumulated in one set, parser vectors sized from the source, literal
  fast path, names resolved only with named groups, pattern units from the flat string.
- D3136: RegExpCreate(proto, regexp, flags) shares the program when the flags match (matchAll,
  `new RegExp(re)`); the flags getter returns [[OriginalFlags]] for a pristine RegExp;
  matchAll's pristine path skips SpeciesConstructor and Construct; RegExp(string, string)
  skips the unobservable steps.
- D3137: `%RegExpExecOwn` runs exec/test/RegExpExec when `exec` is the original data
  property and lastIndex an own Number, else declines (undefined).

Later (same log):
- D3138: the compile cache (coordinator-approved): 8 entries in `Intrinsic::RegExpCache`,
  exact (source, flags) equality on flat strings, sources up to 256 units and programs up to
  1024 words: at most ~36 KiB alive, typically 3-4 KiB. Plain-dump fuzzer regress: clean
  without --lazy; with --lazy one finding (seed 342347, "TypeError: not an object" at plain
  load), which main's difftest binary reproduces identically: not this branch.
- Escaped `source` returns the source itself when nothing needs escaping.
- Allocation ceilings in tests/allocs.rs lowered to today's counts (7, 22, 30, 32, 5, 15).
- Tests: regexp unit tests for the search (stackless attempts, anchors, fuel additivity over
  Latin-1/UTF-16/code points/line starts), corpus case builtins/regexp-cache-and-own-exec
  (cache hits and misses, shared programs with other flags, matchAll's copies, own and
  prototype exec, lastIndex objects, frozen RegExps, line terminators, one- and two-byte
  replace). test262 built-ins/RegExp and String/prototype: 2918/2918; difftest builtins and
  codemode with --suspend-every 7 --gc-stress --determinism: clean; e2e plain and dump.
- Size (D3139): +2,210 gzip shipped, +2,128 slim; ceilings raised by that (approved).
- Guide: no page describes the matcher's internals or these fast paths; continuations.md,
  suspension.md, limits.md still read true. No guide change.

## Follow-ups
- Compiling distinct patterns is still ~2x QuickJS: parse builds a tree (a node per
  character, dropped right after); literal runs as one node, or emitting without a tree, is
  the next step.
- Plain-dump fuzz --lazy finding, seed 342347 (pre-existing on main).
- benchpage's ops.ts could not be read at load 90-190 (zeros, inverted ratios); rerun it
  quiet for the record.

## Result
Final, wasm instructions per repetition (m.py, slope of 1 vs 4 reps, n = 5; QuickJS =
Bellard 2026-06-04 wasm), base 8f2f839f -> branch:

| workload | base | branch | QuickJS | djs/QuickJS before -> after |
| --- | ---: | ---: | ---: | --- |
| replace /\s+/g | 895 | 424 | 370 | 2.42 -> 1.14 |
| exec /^([^:]+):(\d+):(.*)$/ per line | 734 | 318 | 408 | 1.80 -> 0.78 |
| matchAll /^export .../gm | 382 | 236 | 245 | 1.56 -> 0.97 |
| test /\b(TODO\|FIXME)\b/ per line | 399 | 222 | 385 | 1.04 -> 0.58 |
| new RegExp('report\|metric'+i, 'i') (distinct) | 261 | 199 | 100 | 2.62 -> 2.00 |
| new RegExp('report\|metric', 'i') (repeated) | 201 | 83 | 70 | 2.87 -> 1.19 |
| match /function \w+/g | 235 | 123 | 244 | 0.96 -> 0.50 |

wmeasure (whole runs, Minstr): regexp-compile 4,239 -> 2,179 (-49%), pi-like 5,096 -> 4,969,
strings 1,979 -> 1,953, loop unchanged. Decisions D3135-D3139. Size +2,210 gzip (D3139).
Known gaps: distinct-pattern compile 2x and replace 1.1x QuickJS (see Follow-ups).
