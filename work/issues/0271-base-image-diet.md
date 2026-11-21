# 0271: A smaller base image: idle at QuickJS parity (320 KiB)

status: in-progress
agent: pstart
branch: issue/0271-base-image-diet
depends-on: 0265
touches: research first; then the base image's composition (crates/djs-prelude, builtins/ build options), the template layout for privileged code (heap/cells.rs FuncTemplate and its readers, bytecode/template.rs), the js reader layout (regenerated); coordinate every interp reader change through the coordinator
guide: heap.md, snapshots.md, builtins.md if the realm changes
decisions: D3420-D3429

## Goal
Idle is 384 KiB (stack 64 + data in 2 pages total 128 KiB; dlmalloc's primed page 64 KiB;
the arena 3 pages for a ~184 KB base). QuickJS is 320. The arena fits in 2 pages if the base
(plus the per-VM tail and the first session cells) fits in 128 KiB: ~56 KB less base.
Where it goes (IDEAS.md 0.3 and 6): 863 templates with 48-byte headers (41 KB) over 39.5 KB
of bytecode, 863 function objects (20.7 KB), 25.7 KB of name strings, 15 KB of PropTables.
Options: a compact template kind for privileged code (16 B header: −27.6 KB), function
objects reified on first read (−20.7 KB, IDEAS idea 3 stage 2: needs a tag test on prefix
reads), families no codemode run touches left out of the shipped base or loaded as a second
chunk on first use (typed arrays, iterator helpers, Proxy/Reflect, DisposableStack, WeakRef,
...; docs/empty-heap.md and docs/frozen-builtins.md 6), shorter name strings.
Research first: a script over base.img computing the image size under each rule, the per-row
memory effect (does the arena really drop a page, given the arena's start and growth rules,
D3140/D3141), the CPU cost of each (wmeasure), the size cost; report the ranked options to
the coordinator before building.

## Acceptance criteria
- [ ] the ranked options with numbers in the issue
- [ ] built: idle and every codemode row −64 KiB (or the reason it can't be done) at no measurable CPU cost; the codemode-memory leg updated
- [ ] semantics unchanged for scripts (frozen builtins mode included); images and dumps consistent; size reported
- [ ] gate green; decisions recorded; guide updated

## Notes
- pcall (0268) works in run_hot and the call path, which read FuncTemplate fields at
  frame entry: any layout change there goes through the coordinator and their twin check.
- pidle (0270) adds ops and changes class codegen; pgc (0269) strings and atoms.

## Shared context (all of 0256-0260)
The owner asked to push both memory and speed as far as they go (2026-11-21). Background,
fresh profiles and the ranked idea list: work/scratch/perfreview/ of the primary checkout
(BRIEF.md, IDEAS.md; prof.ts and cat.py profile the codemode workloads in wasm). Read
IDEAS.md's section for your issue first; its numbers are estimates, not measurements.

Five issues run in parallel (0256 per-VM/per-byte costs, 0257 idle memory, 0258 GC,
0259 builtin calls, 0260 byte kernels). Stay in your `touches:`; announced overlaps are in
each issue's Notes.

Measuring (D016: wasm32 under node is the yardstick):
- run time and memory per workload: build the shipped module in your worktree
  (`scripts/build-wasm.sh`, or `--keep-names` for profiles) and run
  `DJS_WASM=$PWD/target/wasm32-unknown-unknown/wasm/djs_wasm.wasm node js/bench/engines/run.ts --engines djs --reps 25`
  before and after, on main's build and yours, at load below ~4 (`uptime`; say the load).
  Memory claims: scripts/perf/mem-perturb.sh (D2048).
- instructions: scripts/perf/wmeasure.py / wmicro.py (TESTING.md "Instruction ratios").
- Report per workload: ms and MiB, main vs branch, and the module's gzip bytes.

Size: the shipped module is ~1.3 KB gzip under the D020 budget (ceilings, D3266). Report
the bytes of every change; find offsets where you can. If a measured win needs more room
than that, stop and tell the coordinator with the numbers: do not raise ceilings yourself.

Never weaken determinism (collection points a function of the program, D1710), suspension
anywhere, images, or the unsafe rules (AGENTS.md).

## Log
### Research (2026-11-21, main 2fec34fb; tools in work/scratch/0271/)
Idle today: arena 189,656 B = prefix 174,184 (`BASE_END`) + the image's per-VM part 9,936
(realm, global object and its 67-entry table, stack, shape transitions, VM atoms) + the
sandbox's 5,536 (adding `console` regrows the full global table: cap 134 + 512-bucket index,
4.2 KB, the old 1.6 KB left as garbage). Two pages need <= 131,072: -58.6 KB.

What the prefix is (basediet.py): 863 templates 85.4 KB (headers 41.4, code 39.6, handlers
1.1, padding 3.2; one shared CONSTS pool, IC/INFO/CAPTURES all empty), 799 strings 25.7 KB
(10.2 KB of text; 196 messages 8.2 KB), 863 function objects 20.7 KB, PropTables 14.9,
U32Array indexes 9.4 (already 3/4 load, D2450), ValueArrays 10.6.

Families (famvar.sh: the full base built with one builtins file left out, exact bytes):
typedarray 15.9, dataview 5.2, arraybuffer 3.5 (together 25.5), iterator helpers 11.0, proxy
7.6, disposable+SuppressedError 6.6, web (DOMException, atob/btoa) 6.0, decorators 4.2,
asyncgen 3.1, weak 2.2, reflect 2.1, fromasync 2.1, symbol 1.8, generator 1.8, weakref 1.5,
bigint 1.3, uri 0.8; date 8.2. The slim set (`@slim-omit`) together 50.4 KB.

Page check (rows.ts; full module with families left out of the prelude, outputs identical):
| base | idle | seq | fanout | media | inspect | loop | text | json | heavy |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| main 184.1 KB | 384 | 512 | 1472 | 768 | 448 | 704 | 896 | 1216 | 3968 |
| slim set -50.4 | 384 | 448 | 1536 | 768 | 384 | 640 | 896 | 1152 | 3904 |
| + Proxy -58.1 | 320 | 448 | 1536 | 768 | 384 | 640 | 896 | 1152 | 3904 |
| + Reflect, weak, fromasync -64.7 | 320 | 448 | 1536 | 704 | 384 | 640 | 896 | 1152 | 3904 |
| -58.1 and the eighth rounded down | 320 | 448 | 1472 | 768 | 384 | 640 | 832 | 1152 | 3904 |
- -58.1 KB leaves the idle arena at 129,840: 1.2 KB of margin.
- Text, media and fanout end above 8 arena pages, where D3140's step (`cap + cap/8`, rounded
  up) is 2 pages, so a 58 KB smaller base saves a page only by luck there. With the eighth
  rounded down to pages (floor-round.patch, pgc's rule), text drops too. Fanout's +64 is the
  past-target rule's path (cap 21 vs 20 pages with 59 KB less live data).
- Instructions (icount.sh, --wasm-sync-tier-up, -58.1 KB vs main): sandbox create -11.8%,
  sequential -1.5%, inspect -1.7%, text -0.5%, json -0.3%. Module -14.5 KB gz (the base
  section), the families' Rust natives still in.

Ranked options (KB of idle arena; CPU; size; effort):
1. Leave the cold families out of the shipped module's base (slim set + Proxy, -58.1;
   + Reflect, WeakMap/WeakSet, fromAsync -64.7): idle 320 alone; CPU better (above); -14.5..-16
   KB gz; days (a prelude/feature set). Semantics: those globals are absent in that module, an
   owner decision (D2411's slim module already does it for the slim set).
2. The same families as a second chunk loaded on first use: same memory for runs that don't
   touch them, CPU ~0 if a lazy global rides the accessor branch of property reads (a prop
   flag) and enumerations test an object flag; but Iterator helpers hang off
   %IteratorPrototype% and fromAsync off Array, prefix holders that cannot be written (keep
   them in the base or as stubs, -39 KB then); chunk cells sit above `BASE_END`, so frozen
   mode, the registry/dumper and images need a second notion of "builtin". 2-3 weeks, loader
   +2-4 KB gz.
3. Global table headroom in the image (room for the host's globals, index sized for it):
   -4.1 KB per VM, no CPU, small. Worth it with any choice (doubles the idle margin).
4. Compact privileged templates: 32-byte header (CONSTS from the realm's pool, code at +32,
   frame entry selects both on a FLAGS bit): -13.8 (-9.7 after 1); ~+4 instructions per frame
   entry, pcall's path. 24-byte (counts as bytes): -20.7, a layout branch at frame entry.
   IDEAS' 16-byte one breaks D2448 (a closure's ELEMENTS is its template, +8 must read 0).
5. Function objects on first read: -20.7 (-14.5 after 1) minus a per-VM identity table; a tag
   test on every value read from a prefix holder (proto IC hit, prop_read, enumerations,
   dumper); a token that escapes is a heap-safety bug under unsafe-heap. After idea 3's
   tokens if at all; 2-3 weeks.
6. Prefix hash indexes as u16: -4.7; a width test on prefix-table probes (IC misses, atom
   interning).
7. Prefix strings without the cached HASH: -3.0; error messages packed in one cell: -3..4.
   Moving messages to module data does not help: data pushes dlmalloc's room (the floor) out.
Without families no combination gets there: 3+4(24 B)+5+6+7 = -53 KB < -58.6.
Recommended: 1 (or 2 if the families must stay) + 3: idle 320 with ~5 KB margin, plus an idle
row in the codemode-memory leg. Every row -64 KiB is not reachable from the base alone (text,
media, fanout, above); it needs D3140's step rounded down (pgc).

### Building
- Coordinator (2026-11-21): (a) the families are the owner's call; pi needs Proxy, Reflect
  and Int8Array (lockdown reads %TypedArray%), so price a variant that keeps them with 3 and
  4. (b) floor-round.patch goes to pgc. (c) 5 is dropped; 4 needs slope.py on calls and
  higher-order first, and pcall's review. Build 3 and the leg's idle row now.
- D3420 (3): global table room 16, image +256 B, idle arena 189,656 → 185,680; loop
  704 → 640 KiB, other rows unchanged; idle row in codemode-memory. Guide: snapshots.md.
- Merged D3420 at e1d853e5.
- Variant for the owner, keeping Proxy, Reflect and WeakMap/WeakSet, with typed arrays
  reduced to a %TypedArray% and an Int8Array that throw (pi's lockdown reads
  `Object.getPrototypeOf(Int8Array)`): the slim set out with D3420 gives an idle arena of
  135,464 B (w-slimset). The stub would add about 0.6 KB; that is an estimate, because bgen
  compiles every file, so a stub can't replace typedarray.ts in an experiment. That comes to
  ~136.1 KB, still 3 pages. Closing the last ~5 KB:
  (i) 4 as a 32-byte compact header: -9.7 KB (605 templates), ~126.4 KB, 4.7 KB margin. Cost
      (compact-cost.patch: the frame entry and return selects code offset and pool on a
      FLAGS bit, no template compact yet; slope.py -n 2M, A/A noise up to 1%): call +2.0..2.2%,
      method +1.8..2.1%, reduce +0.3%, forEach +0.7%. Every call and return pays it.
  (ii) one 40-byte header for every template, so DATA stays a constant and frame entry
      doesn't change: CAPTURES goes into the cell header's free u16, INFO into a trailer,
      IC_COUNT folded into the IC slot or into FuncInfo. -4.8 KB; plus u16 prefix indexes
      -3.9 KB gives ~127.4 KB, 3.7 KB margin. Costs: closure creation (INFO via the cell
      size), the first IC fill, and a width test on prefix-table probes. Not measured.
  (iii) u16 indexes -3.9 KB plus prefix strings without HASH -2.4 KB: ~129.8 KB, 1.3 KB
      margin. Too thin.
  Rows would be as in the -58.1 row of the table (idle 320; seq, inspect, loop, json and
  heavy -64; text and media 0; fanout +64 unless pgc's rounding lands).
- (ii)'s costs off the frame-entry path, measured on a prototype (offpath-cost.patch: every
  template gets an 8-byte trailer at the cell's end; the first IC fill reads IC_COUNT from it;
  `template::child` reads a 0 from it and adds it to INFO's address, one load more than a
  real trailer; `Heap::probe` and the atom lookup test a u16 flag on the index and pick the
  bucket width, the flag never set). slope.py -n 1M -r 3, --wasm-sync-tier-up, load 20-30,
  A/A within 0.8%:
  | case | A | B | delta |
  | --- | --: | --: | --: |
  | closure creation (`const f = (x) => x + i` in the loop) | 2389 / 2375 | 2425 / 2431 | +1.5 / +2.4% |
  | `in Math` (prefix-table probe) | 2186 / 2178 | 2204 / 2190 | +0.8 / +0.5% |
  | `Object.hasOwn(Array.prototype, k)` | 3869 | 3866 | 0 |
  | call, method (control) | 828, 1184 | 827, 1188 | -0.1, +0.3% |
  Compile (cab.ts, wall, sequential and text bodies): +1.4%, +1.7% (the atom lookup's
  width test on every identifier, plus 8 bytes more per template written).
  So (ii) is not free either: ~2% on closure creation (fewer than 36 instructions per
  closure; the measurement is an upper bound by one load) and ~1.5% on compile. The u16
  index part costs about half a percent on prefix probes. An alternative with no read-path
  cost does not exist for the template header: everything left after CAPTURES (into the
  header's free u16) is read at frame entry (FLAGS, counts, CODE_LEN, CONSTS), by IC ops (IC),
  or at closure creation (INFO, NAME, LENGTH).

### pi end to end: a0d85b06 (before the push) vs main 968f3465
Main module (the dump module where a dump is involved), js/pi/vm.ts of each tree, pi-private's
PRELUDE_SOURCE (prelude-next for the next shape), two tools; work/scratch/0271/pib.ts (median of 40,
the trees interleaved, load ~6), instructions: the whole node process over N sessions minus N=0,
--wasm-sync-tier-up, min of 2. Memory: the measured session in an instance of its own, a fresh
module (no pool), after a first session that is kept open.
| case | old ms | new ms | old M instr | new M instr | new/old | old KiB | new KiB |
| --- | --: | --: | --: | --: | --: | --: | --: |
| start, prelude image reused (`return 1`) | 0.17 | 0.15 | 5.89 | 5.57 | 0.946 | 768 | 576 |
| start, prelude run (no image) | 4.14 | 3.96 | 109.6 | 103.9 | 0.948 | 1472 | 1280 |
| tool-heavy (61 calls, 120 KB of results) | 1.99 | 1.93 | 56.1 | 53.6 | 0.956 | 1216 | 1024 |
| next shape, tool-heavy, dump module | 1.62 | 1.57 | 44.5 | 42.6 | 0.956 | 1152 | 1088 |
| next shape, tool-heavy, main module | 1.62 | 1.47 | | | 0.913 | 1088 | 896 |
| next shape, dump + resume at every idle point | 64.2 | 61.8 | 1544 | 1529 | 0.990 | 2816 | 2880 |
| 12 x console.log of a 1 MiB `"x".repeat` string | 4.78 | 16.3 | 111 | 359 | 3.23 | 7488 | 5760 |
- Every pi session is 64-192 KiB smaller and 4-9% cheaper. The exception is joining a
  repeat-built rope, which is 3.2x slower: D3400 (0269) no longer flattens it, and
  `collect_range` visits ~1M shared one-character leaves per join (26.9M vs 6.3M instructions
  per `[s].join(" ")`; appended ropes got cheaper, 1.26M → 0.47M). Reported to the coordinator
  for pgc. The dump+resume row's memory is the last of the instances the session moved
  through, so it shows the pool more than the session.
- pi's own tests (js/pi/pi-tests.ts, main module): the sum of test durations is 15.15-15.19 s
  old against 16.92-17.30 s new (two runs each). The largest increases are the output-limit
  tests (1 MiB repeat strings, the regression above, +92..+183 ms each) and wall-clock
  timeout tests that are load-bound. Instructions per interrupt poll (FUEL_PER_POLL) fell
  457K → 346K in a tight loop (pcall's 0268), so budgets poll ~30% more often per second of
  CPU: they mean less CPU time than before.
- What pi's path is made of on main (cpu profiles, names builds, prof-*.cpuprofile):
  - start with the image reused (0.15 ms): instantiating 18% (Instance 12.9%, djsExports
    5.3%; the pool keeps 2), **the image cache key 16%** (`preludeKeyOf` JSON-stringifies the
    whole prelude every start, since pi passes a new `data` per run and the WeakMap misses,
    then the Map hashes that string), restoring the image's zero-fill (`Vec::resize`) 10.6%,
    V8 GC 4.5%. The engine itself (compile and run of the body) is under 10%.
  - tool-heavy (1.9 ms): run_hot 20%, the JS host and the test's tool ~15% (answer, tool,
    Encoder, outWords, Reader), JSON parse of the results ~6%, collection 2.6%.
  - dump + resume (~7.5 ms per idle point, 8 points): writing the dump 46%
    (`Dumper::finish`: plain::write 18%, react_upto 12%), loading it 43% (`loadPlain`:
    compile 32%, of which parse 18%). It scales with the data the session holds (the
    read results are string literals in every dump).
- Cheap wins in js/pi found (not built, outside 0271's touches): key the image cache on the
  prelude's hash kept per prelude string instead of JSON of the whole text (-16% of a
  cached start).

## Follow-ups

## Result
