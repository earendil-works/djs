# 0162: Act on pico's speed findings

status: done
agent: speed, speed2
branch: issue/0162-pico-speed
depends-on: 0152, 0154, 0151 L4 (merged first)
guide: docs/guide/interpreter.md / builtins.md as affected
touches: crates/djs-core/src/interp/ (run_hot callback returns, fuel accounting), crates/djs-core/src/builtins/ (reduce and the callback builtins' TS), crates/djs-core/src/vm/ (host result path into heap strings); perf (0159) prunes @native/@fast twins in builtins/, bloat (0151 L3) is in compiler/ heap/: coordinate via the coordinator

## Goal
docs/lessons-from-pico.md and 0154's follow-ups, the speed items:
1. **pico's tree benchmark** (children + map/reduce): 665 ms vs quickjs-wasi 310. 0154 left:
   an argument-count intrinsic so `reduce` needs no rest array and second frame; callback
   returns on short arrays handled inside run_hot (after L4's encoding is on main); the
   collector's cost on long-lived trees (measure what share remains after the first two).
2. **Fuel only at calls and backward jumps** (pico's rule, lesson 7): measure what charging
   fuel per instruction costs today, and whether charging at calls/backward jumps (with a
   per-block bound so determinism and the suspend-anywhere guarantee hold, G5) pays. Fuel
   counts are part of determinism (the same history gives the same counts), so the rule
   change must be global and recorded.
3. **Large results straight into their final heap string** (lesson 8): host results and
   large JSON/string results skip the staging buffer.
Use scripts/perf/instr.py (D2309) for splits; wasm instructions on all workloads, the pi
bench and pico's rewritten cases (work/scratch/0152, 0154/w). No workload slower than
+1%; size ratchet respected or trades recorded.

## Log
- Scratch: work/scratch/0162 (wbuild.sh builds the perf wasm into a file; hoststr.ts times
  host string results; fuel-safepoint.diff the item 2 experiment; fuel2.txt its numbers).
- Item 2 (D2550): experiment build charging fuel only at calls, returns and taken backward
  jumps in run_hot. ≤7% fewer wasm instructions on interpreter-bound loops (loop, tight,
  prop1), 2–4% on calls/arrays/foreach, ~0 on pi-like, JSON, strings. Coordinator: not now
  (it would change G5 and every fuel count); recorded with the full table.
- Item 3 (D2551): `vm_string_reserve` + `stringInPlace` (js/src/wire.ts) for host strings
  ≥ 1024 units whose wire encoding shows ASCII: copied from the wire bytes straight into
  the reserved Str8, no staging, no transcode. 10 MiB result: 21.4 → 11.4 MiB wasm memory,
  ~1.5–2x faster; non-ASCII unchanged (encoding into the reservation first cost them
  +0.7 ms/MiB and +1.3 MiB, dropped). Size +69 B gzip (455,571, ceiling 455,600). Tests:
  djs-wasm `strings_written_in_place`, js sandbox test for the in-place and wire cases.
  Guide: host.md ("Values in") updated; docs/host.md export table.

- Item 1 (D2552): `%ArgumentCount` leaf; reduce/reduceRight take `initialValue?` and run
  in one frame each; `%EachDense` mode 7 keeps the accumulator in its THIS slot, a box
  only for non-empty arrays (ArrayCreate(0) and ArgumentCount are tier-0 hits). A first
  version with `ArrayCreate(0)` + CreateDataPropertyOrThrow for the box was +7–9% on
  reduce1/2 (two general-path exits): dropped. vs main: tiny-reduce0 −46%, reduce1 −25%,
  reduce2 −23%, tree-children −11%; 26 other workloads within ±0.8%; size −70 B gz.
  Corpus: builtins/array-reduce-args.js (qjs expected; --suspend-every 1, gc-stress,
  determinism OK). test262 Array/prototype/reduce{,Right}: 514/514.
- scripts/perf/instr.py: its run_hot anchor moved with D2443 (`let mut op`), fixed.
- Tree profile (wasm cpu-prof after D2552): run_hot 26%, collector 26%, Each machinery
  ~5%, closure creation 6–8%, map's species/ArrayCreate fixed cost. Coordinator: skip the
  run_hot callback-return change (≤2–3%, recorded in D2552); closure creation goes to
  bloat (0151 L3 step 3); species/ArrayCreate tweaks in builtins/fast.rs after 0159.
- Guide: builtins.md (`%EachDense` paragraph: reduce's accumulator and ArgumentCount).

- speed2 took over (speed's context ran out). Rebased twice onto main (0163 SipHash,
  0165 slices: stringify reads strings through `contiguous` now).
- Item (b) (D2554): ArrayCreate's length set directly (`new_array_with_length`), species
  checks skipped when Array and its prototype are frozen builtins. vs main (default
  tiering, n=5, outliers n=9): tiny-map0 −5.5%, tiny-filter0 −5.6%, tiny-map1/2 and
  tiny-filter1/2 −2..−4%, tree-children −1.7%, tree-filter −2.1%, the rest ±0.8%; pi bench
  −0.3%; −88 B gz. A first measurement run beside a cargo build showed +1.4–2.2% on
  json-text, json-date, tiny-alloc/base: re-run quietly, ±0.7% (tier-up timing under
  load; don't build while measuring default tiering).
- Item 2 (D2553): JSON.stringify spills its buffer into the result's heap string every
  4096 units (in place while it is the arena's last cell, else a new piece; a rope at
  the end). The 0116 concern (allocations in between) only costs a new piece. Peak
  memory: json-date 5.14 → 3.50 MiB, json/jsonh −0.4, json-replacer −0.2, an 8 MB result
  34.2 → 25.9; instructions ±0.8%, pi bench −0.3%. Size: the first version (long strings
  in parts, in-place two-byte appends, the room check kept) was +890 B gz, over the
  ceiling; trimmed to +273 (+185 with D2554). After the next rebase the slim build was
  6 B over: the closing-bracket check point went too (slim 4 B under its ceiling). Long-string parts
  also made a 4 MB `repeat` string peak higher (13.9 → 15.8 MiB): dropped with them.
  Corpus: builtins/json-stringify-large.js (qjs expected; stress, determinism and
  --suspend-every 1 OK); unit test json::tests::stringify_large_output_spills_into_the_heap.
  test262 built-ins/JSON 163/163, Array map/filter/slice/splice/concat/flat(Map)/species
  all pass.
- Guide: gc.md (the D2541 trade-off note: the stringify cost is gone), builtins.md
  (frozen builtins: the species shortcut).
- Item 3 note, the collector in pico's tree case: see Follow-ups.

## Follow-ups
- **Promotion of long-lived young data (the collector's ~26% of pico's tree case).**
  tree-children builds 131,071 nodes (an object and a `children` array each) once and
  walks them 5 times; each walk allocates a `map` result and closures per node, all
  garbage at once. The threshold lets the heap grow by its live size, so every collection
  comes after about one tree's worth of garbage and marks and slides the whole tree: the
  collector's share stays ~26% whatever the tree's size. The old region doesn't help: it
  is only the sealed setup state (D2041/D2042). What promotion would need:
  - a rule that finds long-lived data. D2046's aging (promote the range that survived two
    collections) is the prototype;
  - minor collections still walk the whole old region for old→young references (no write
    barrier, D2044). A walk is cheaper than mark+slide (minor 0.08 ms vs full 0.18 ms on
    the base) but scales with the tree, so promotion would cut maybe half of the
    collector's share (~10–13% of the run), not all; the rest needs a card-marking
    barrier, which D2044 rejected;
  - full collections every 16th minor still trace everything.
  What's different from D2046: there the cells that survived two collections died soon
  after (pi-like's per-task data, strbuild's replaced buffers, json-replacer), so dead
  promoted data stayed until the next full collection and raised peaks by 2–5 MiB. Here
  the promoted data really is long-lived. The difference can be measured at the
  collection: promote an aged range only when most of it (say ≥ 90%) survived its second
  collection. The tree's would; per-task data and replaced string buffers wouldn't.
  Codemode has the same shape whenever a large parsed tool result is kept while a script
  maps and filters it. To measure: tree-children/tree-filter, pi-like, strbuild and
  json-replacer peaks (mem-perturb) and instructions.
- JSON.stringify of a rope reads it through `contiguous`, which flattens it (a full copy
  of a large rope, e.g. from `repeat` or `+=`). Quoting from the rope's chunks would save
  that copy; it needs a chunk cursor that survives the spills' heap writes.
- Large two-byte JSON.stringify results end as ropes of 4096-unit pieces (D2553); an
  in-place two-byte append (~200 B gz) would keep them flat if that turns out to matter.

## Result
Items 1–3 of the goal and the follow-up (b):
- reduce/reduceRight in one frame via `%ArgumentCount` (D2552): tiny-reduce0 −46%,
  reduce1/2 −23–25%, tree-children −11%.
- Fuel at safepoints only (D2550): measured (≤7% on interpreter-bound loops), not
  adopted (it changes G5 and every fuel count).
- Large ASCII host strings written straight into a reserved heap string (D2551): a
  10 MiB result 21.4 → 11.4 MiB wasm memory, 1.5–2x faster.
- JSON.stringify builds large results in their heap string (D2553): json-date 5.14 →
  3.50 MiB peak, an 8 MB result 34.2 → 25.9 MiB, instructions ±0.8%.
- Species/ArrayCreate shortcuts (D2554): tiny map/filter −2..−5.6%, tree-children
  −1.7%, tree-filter −2.1%.
- No workload or pico case slower than +1% (re-run at n=9 where a first run showed
  more); pi bench −0.3%. Size within both ceilings (this round +185 B gz).
Known gaps: the collector's share of long-lived trees (follow-up with what's needed),
stringify still flattens rope inputs, large two-byte stringify results are ropes, the
run_hot callback-return change was skipped (≤2–3%, D2552).
