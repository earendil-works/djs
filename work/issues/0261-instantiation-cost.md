# 0261: Instantiation and sandbox setup costs

status: done
agent: pstart
branch: issue/0261-instantiation-cost
depends-on: 0256
touches: crates/djs-wasm (exports, the ABI, data segment layout), js/src (sandbox.ts, base.ts, the ABI wrappers, layout of host setup), js/pi/vm.ts and js/pi/engine.ts (entropy pool, setup calls), js/test
guide: host.md, js-api.md if the ABI or setup changes
decisions: D3320-D3329

## Goal
0256 found that ~100 µs of a ~140 µs new sandbox is V8 instantiating the module, and
named the parts (its Result and Follow-ups):
1. Exports: ~0.3-0.8 µs each per instantiation, 65 of them; a stripped-exports module cut
   instantiation by about a quarter. djs is unreleased (D024): change the ABI in place.
   Fewer exports (e.g. rarely used ones behind one dispatch export, or folded calls),
   with js/src, js/pi and the tests updated.
2. The 97 KB data segment: about another quarter of instantiation. Find what in it a
   typical run never reads (cold Unicode / case / regexp property tables, panic and format
   strings) and keep it out of the segment that every instantiation copies, e.g. coded in
   a custom section and materialized on first use, the way the base image is. Measure
   instantiation, idle memory and the first use's cost. Coordinate with 0257 (pidle),
   which changed the stack/data layout and dlmalloc's first segment.
3. Sandbox setup (~30 µs): batch the console host-function setup (~9 small wasm calls)
   into one; the arena's growth by single pages costs ~2 µs per memory.grow call (4-10
   per run): grow in larger steps where it does not add touched pages.
4. js/pi/vm.ts: use the entropy pool for its per-VM seeds (as 0256 did for Sandbox).
5. Rope interning compares through per-unit iterators (~0.8% of sequential): flatten or
   compare by slices.
Target: a new sandbox well under 100 µs at a quiet load, idle memory not worse.

## Acceptance criteria
- [x] new-sandbox time and small-workload run times, main vs branch (interleaved, as in 0256)
- [x] the ABI change carried through js/src, js/pi, djs-embed if affected, and tests; layout/ABI version bumped as the repo requires
- [x] size reported (the module is near the D020 budget: prefer changes that shrink it)
- [x] gate green; decisions recorded; guide pages updated

## Notes
- 0257 (pidle) merges an elastic stack (a new `djs_stack` export, relocation in
  sandbox.ts/trap.ts) and the data-page layout: rebase onto it before touching those files.
- 0259 (pcall) and 0260 (pscan) work in djs-core; 0258 (pgc) in the heap.

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
- Data segment anatomy (wasm-ld `--Map`, work/scratch/0261/map.txt): 99,123 B, of which the
  Unicode tables are 58,299 (props DATA 15,275, DECOMP_TARGETS 9,623, DECOMP_ENTRIES 9,212,
  GC_RUNS 5,202, ID_START 2,948, SCRIPTS 2,816, case tables ~6 KB, CCC ~2 KB, ...),
  intrinsic NAMES + NAME_ENDS 8,152, the rest small.
- Instantiation anatomy (strip2.py truncates the data segment; inst5.ts): no data 73 µs,
  16 KB 96, 48 KB 101, full 104. Most of the segment's cost is the first touch of the
  instance's memory (an empty-data instance plus one byte written: +12-18 µs), which every
  sandbox pays anyway when the base is copied in; shrinking the segment saves ~1.3-2 µs per
  16 KiB page.
- D3320 one export: instantiation 81-85 → 59-65 µs. The JS wrapper first cost it back:
  the per-instance loop (~2-4 µs, `OPS.indexOf` per op removed) and elasticStack's
  `Object.assign` into a null-prototype object (~8 µs) — fixed in place (D3321). Profiles
  attribute the V8 GC that frees old instances' memories to whatever allocates (TextEncoder
  in stageText here), so blocks of one build at a time are the fair comparison.
- D3321: encodeInto for short names; pi's entropy pool. D3322: rope vs flat compare.
- Not done: batching the console setup (9 calls ~1 µs each, mostly first touch of the
  arena: a batch would pay that too); growing memory in larger steps (raises linear memory,
  the reported metric, for ~2 µs per avoided grow).
- Final (work/scratch/0261/final-ab.txt; main da9d50cc vs branch, each with its own JS,
  800 interleaved runs, load ~12): new sandbox 0.0985 → 0.0850 ms (−13.7%), sequential
  −6.9%, inspect −3.8%, media −5.0%, fanout −2.1%. Blocks of 2,000 sandboxes per build
  (sb.ts): 141-151 → 116-127 µs. Memory unchanged (no layout change). Size 429,363 →
  429,049 B gz (−314: one export −499, rope compare +185). Guide: host.md (operations, one
  export), status.md (wasm/ops replaces wasm/exports); docs/host.md ABI section.

## Follow-ups
- **Cold Unicode tables out of linear memory (item 2; its own issue, pending the owner's
  size decision).** Design note:
  - Why: stack 64 KiB + data 99,123 B = 164,659 B, the third page. Two pages need
    ≤ 131,072 B: 33.6 KB less data (plus whatever dlmalloc's seed wants, D3281). Prize:
    −64 KiB on every row (idle 448 → 384 KiB); instantiation only −5-8 µs (see Log).
  - Families: regexp properties (`\p{…}`: props DATA 15,275, GC_RUNS 5,202, SCRIPTS 2,816,
    BINARY 600, GENERAL_CATEGORY* 336, STRINGS 120, ... ≈ 24.5 KB) and normalization
    (DECOMP_TARGETS 9,623, DECOMP_ENTRIES 9,212, DECOMP_INDEX 1,120, CCC_RUNS 1,612,
    CCC_VALUES 403, COMPOSE_EXCLUDED 324, NFC_QC_* 488 ≈ 22.8 KB). Together ~47 KB.
    Case tables (~6 KB) and identifier tables (ID_START/ID_CONTINUE_ONLY 4.5 KB) stay:
    the lexer (non-ASCII identifiers) and every non-Latin-1 case mapping read them, and
    those paths have no clean stop.
  - Who reads them at run time: props only `property::lookup` in the regexp parser
    (regexp/parser.rs:1157): the compiled class keeps its ranges, matching never reads the
    tables. So `\p` can stop only at a regexp compile (a literal's first evaluation,
    `new RegExp`, `RegExp(...)`), the paths that already stop for NEED_STACK (D3280).
    Normalization: `String.prototype.normalize` (builtins/string.rs:552-582,
    `is_normalized_quick` and `normalize`); nothing else (no localeCompare use). Case
    mapping does not touch them.
  - Protocol NEED_TABLES, modeled on NEED_STACK: the reader finds its family missing
    before any effect and returns a fault; `vm_compile*` returns a distinct code and
    `vm_run` a status with the pc unchanged; the host copies the family's coded bytes from
    a custom section (`djs_props`, `djs_norm`, like `djs_base`, D2541) into
    `djs_tables_ptr(family, len)`; djs-wasm decodes and keeps them per instance and hands
    djs-core a slice through the Heap (no unsafe, no global); `vm_run(fuel 0)` re-runs the
    instruction. Native, slim and test builds keep the statics (cfg). Deterministic:
    whether a run needs a family is a function of the program.
  - Code estimate +1.0-1.6 KB gz: stop paths in the regexp parser and `normalize`
    (~300 B, much shared with NEED_STACK's), table access through a slice instead of
    statics (~200-400 B: bounds checks the constants avoided), djs-wasm's op, section
    handover and decode (~400 B; lzrc's decoder is already in the module), JS host side
    (sandbox.ts, engine.ts) not in the module. Offsets: the moved tables' own bytes leave
    the module's data (−47 KB raw, ~−20 KB gz of data become ~−15-18 KB gz of coded
    section: the gz total roughly washes; the D020 budget counts the module file).
  - Smaller first step: `\p` alone (~24.5 KB) is ~9 KB short of the line. Candidates for
    the rest: the intrinsic NAMES + NAME_ENDS (8.2 KB; read at run time only for messages
    and by `intrinsic_id`, which the compiler uses for privileged code — dead in the
    shipped module unless something compiles privileged code there; check), the 4.3 KB
    anonymous constant in djs-core (map.txt `.Lanon...566`, unidentified), or a 62 KiB
    stack. Normalization alone (22.8 KB) has the simpler stop (one builtin) but is further
    from the line.
- Batched console setup: only worth it together with something that avoids the arena's
  first touch (it is ~9 µs, mostly page faults).

## Result
Landed: D3320 (one export `djs(op, …)`, ABI 6, js/src/abi.ts `djsExports`, carried through
js/src, js/pi, js/bench, scripts (abidrive.mjs, status.py, sandbox-memory.ts), the docs
site's live.js, tests and the guide), D3321 (elastic wrapper in place, encodeInto for short
names, pi's entropy pool), D3322 (rope/flat compare by chunks). New sandbox −13.7%
(interleaved) / ~−17% (blocks), sequential −6.9%, inspect −3.8%, media −5%, fanout −2.1%;
memory unchanged; −314 B gz. Item 2 (cold tables) is designed in Follow-ups, not built:
its code cost needs the owner's size decision. Items not done: batched console setup and
larger memory.grow steps (measured as not worth it, see Log). djs-embed is unaffected
(native, no wasm exports).
