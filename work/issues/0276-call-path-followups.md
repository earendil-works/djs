# 0276: Call path and cache follow-ups from 0268 and the 0270 sweep

status: done
agent: pcall
branch: issue/0276-call-path-followups
depends-on: 0268
touches: crates/djs-core/src/interp/run (run_hot arms, ic, decline paths, fastpath, iter for collection iterators, spread calls), builtins/fast.rs (FAST flag tests only)
guide: interpreter.md
decisions: D3470-D3479

## Goal
0268's ranked follow-ups and pidle's sweep (re-measure each on current main first):
1. `typeof x === "string"` (1,082 instr) as a loop arm; `typeof o.name === 'string'` 2.1x QuickJS.
2. `Map.get`, `Set.has`, `a.push(i)` (1.2-1.5x): test the template's FAST flag before building a frame.
3. for-of over a Set or Map (3x per element: iter_next_loc declines for collection iterators into step) and `f(...args)` (1.35x).
4. A cache kind for absent properties (`o.missing ?? 1`, 2.1x) without slowing accessor reads (0268 measured a plain miss lookup at +14-25% on accessors: find another shape).
5. The add cache for a key shadowing a writable inherited data property (`e.name = 'X'`, ~10x per store).
6. Optional chaining `DEEP?.a?.b?.c` (1.35x), Math.floor + Math.round (1.5x) if cheap.
Superinstructions (a dispatch is 18 of a simple instruction's 21-29) only as a measured
experiment with the hotcode/slope procedure; report before adopting.

## Acceptance criteria
- [x] per-op instructions vs QuickJS 2026 before/after; workload instructions with the noise control
- [x] the hotcode.sh shared-path check for every run_hot change
- [x] size: ~950 B room under the ceiling, shared (pidle's Set/Map work is waiting for ~865 B of it): prefer size-neutral; report bytes
- [x] gate green; decisions recorded

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

### Re-measured first (main ece737bd, steady state, slope.py --qjs --per 10)
Instructions per operation, djs / quickjs-wasi, both tiered up (the sweep's numbers
included the loop and the tier-up; steady state ranks differently): `m.size` 3547/700
(5.1x), `o.missing` 995/299 (3.3x), own getter 1220/669 (1.8x), `typeof s` 405/252
(1.6x), `o.missing ?? 1` 1150/743 (1.55x), class getter 1591/1052 (1.5x), `s < t`
676/455 (1.5x), `s === "x"` 559/517 (1.1x), `typeof o.name === 'string'` 1078/988,
`m.get(1)` 1068/979, `st.has(1)` 1038/980. Already below QuickJS: `a.push(i); a.pop()`
0.61x, `f()` 0.84x, `p.m()` 0.65x, `Math.floor(i/3)` 0.62x, `Math.floor + Math.round`
0.59x, `d?.a?.b?.c` 0.36x, `f(...args)` 0.61x, `e.name = "X"` on an existing own
property 0.51x; for-of per element: array 0.51x, Set 0.93x, Map 0.93x.
So the list's items 2 (Map.get/Set.has/push), 3 (for-of over collections, spread) and 6
(optional chaining, Math) are at or below QuickJS in steady state and were left alone;
accessor reads, which were not on the list, were the largest gap.

### What landed (D3470, D3471, D3472)
- Cache kinds for absent keys and for accessors (own and inherited); the getter called
  from `decline_fast`. New corpus case language/ic-absent-accessor.js.
- `typeof` as an arm; `===` of flat one-byte strings from the slice; string `<` etc. in
  `decline_fast`.
- Bytes: `exec` reads the top three operands once, `json_regexp_intrinsic` its
  arguments: −793 B gz, which pays for the rest (the issue is −118 B gz).
- hotcode.sh after every step (work/scratch/0276: sumdiff.sh, the hot/ summaries): three
  shapes had to be changed for V8 (a store of the getter in the arm; new kinds read in
  the slice path; a bare constant decline), all recorded in D3470.
- slope.py got `--qjs` (quickjs-wasi as a column) and `--per K`.
- Outside `touches:`: run.rs (`lookup_ic`'s fills, `Exec::hot_getter`, `exec`'s
  preload), json_regexp.rs (argument preload), scripts/perf/slope.py and README.
- Guide: interpreter.md (the caches' absent and accessor kinds; the decline list).
- Checked: lib tests, e2e in four modes, difftest (corpus plain; builtins/, codemode/,
  language/, dquickts/ with suspension, GC stress and determinism; codemode/ lazy with
  plain dumps), test262 language and built-ins (39,251 active, no new failures).

### Not done
- Superinstructions: no experiment was run (the accessor and absent kinds took the time).
- The add cache for a shadowing key (item 5): `e.name = 'X'` on a fresh Error each time
  was not measured; the store on an existing own property is 0.51x QuickJS.

## Follow-ups
1. `@fast` entries for builtin getters (`Map.prototype.size`, `Set.prototype.size`,
   typed arrays' `length`/`byteLength`, RegExp flags): `m.size` is 3,272 instructions
   (4.7x QuickJS) because the getter is a bytecode builtin with two intrinsic calls;
   `decline_getter` already goes through `call`, which takes a `@fast` entry.
2. A getter's frame pushed inside the loop (class and own getters are 1.2x and 1.4x
   QuickJS): needs the call arm's frame push shared instead of copied (~400 B gz as a
   copy).
3. `Map.get` / `Set.has` 1.12x / 1.08x: the `@fast` call's exit and dispatch (D3301).
4. The add cache for a key that shadows a writable inherited data property; measure
   `new Error(); e.name = 'X'` first.
5. Superinstructions as a measured experiment (compiler + op table).
6. `step` pays ~15 instructions per instruction for `exec`'s operand preload (D3471):
   if bytes become available, read only what an arm needs.

## Result
Merged with this commit. Steady-state instructions per operation, main fd3d5759 →
branch, with the ratio to quickjs-wasi (slope.py, both engines tiered up):

| operation | main | branch | x QuickJS before → after |
| --- | ---: | ---: | --- |
| `o.missing` | 1,005 | 266 | 3.37 → 0.89 |
| `o.missing ?? 1` | 1,169 | 430 | 1.58 → 0.58 |
| class getter `p.g` | 1,601 | 1,285 | 1.52 → 1.22 |
| own getter `o.g` | 1,230 | 942 | 1.84 → 1.41 |
| `m.size` | 3,557 | 3,272 | 5.12 → 4.71 |
| `typeof s` | 419 | 159 | 1.64 → 0.62 |
| `typeof o.name === "string"` | 1,085 | 385 | 1.10 → 0.39 |
| `s === "string"` (other cell) | 575 | 140 | 1.12 → 0.27 |
| `s < t` | 688 | 472 | 1.52 → 1.04 |
| own property `o.c` | 92 | 83 | 0.40 → 0.36 |
| method lookup `p.m` | 150 | 149 | 0.47 → 0.47 |

Workloads (wmeasure, M instructions, n=3, spread ≤ 0.6%): prop1 −1.1%, json −0.9%,
json-date −1.2%, pi-like −0.3%, props +1.5% (D3471's preload), the other eleven within
±0.2%. Two builds of the same source measure within the spread. V8's shared path:
dispatch 14, tail 4, simple arms unchanged.
Size: shipped 429,537 → 429,419 B gz (−118), slim 363,475 → 363,363 (−112).
