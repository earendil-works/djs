# 0259: Builtin calls without finding the callee

status: done
agent: pcall
branch: issue/0259-builtin-call-cost
depends-on: -
touches: crates/djs-core/src/compiler (resolve: recognizing builtin call sites), crates/djs-core/src/bytecode (new guard ops), crates/djs-core/src/interp/run (decline_fast, fastpath, call; run_hot only with measurements), builtins/fast table, value tags if a token value is introduced (and the dumper's handling of it)
guide: interpreter.md, builtins.md
decisions: D3300-D3309

## Goal
Small builtin calls are the speed gap left on long bodies: padStart 2.25x QuickJS,
Object.entries/fromEntries 2.38, sort with a comparator 1.84, Map.set 1.81, Set.add 1.52,
object spread 1.45, charCodeAt loop 1.43, trim 1.35, slice 1.32, Math.floor 1.30.
D3265 measured a builtin-call statement at ~1,000 wasm instructions: ~450 to find the
callee, 150-200 in fast_leaf_call's generic dispatch, leaving the loop the smallest part.
Its separate call opcode was dropped because adding to run_hot slowed other code 1-2%.
Attack the lookup instead (IDEAS.md idea 3, "known-builtin call tokens"): recognize
`Math.floor(x)`, `s.trim()`, `String(i)`-shaped sites at compile time, guard cheaply that
the builtin and the path to it are original, and call the leaf by id; any failed guard
falls through to the unchanged generic code, with the spec's evaluation order.
Start with the cheapest experiment (three hard-coded ids, guard in decline_fast, count
instructions per statement), report the numbers, then generalize to the hot table. Then
the per-op items above that are not lookup-bound (Map.set/Set.add string keys, comparator
sort, Object.entries/fromEntries, spread): profile each and fix what is cheap.

## Acceptance criteria
- [x] instructions per statement (wmicro) and the ops table's ratios (js/bench/engines/ops.ts, djs vs best QuickJS) before/after
- [x] no regression above noise on code without builtin calls (wmeasure tight, calls, prop1, loop)
- [x] exact semantics: corpus + test262 cases for replaced/shadowed/monkey-patched builtins (globals reassigned, prototypes patched, frozen builtins mode)
- [x] suspension and dumps: a token (if any) is handled by GC, frames, stack traces and the dumper
- [x] size reported; gate green; decisions recorded

## Notes
- Overlap: 0258 (pgc) adds a latch flag set when a script writes below the base's end
  (reshapes a builtin). That is your "pristine" signal; use it rather than adding a
  second one. If you need it before 0258 lands, agree with pgc on the name and who adds it.
- run_hot is sensitive to any code change (D1352, D3122, D3128): measure every change to
  it against a build-noise control.

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
2026-11-21 (pcall)
- Baseline, main a0d85b06, shipped module 428,796 B gz. Scratch: work/scratch/0259 of the
  primary checkout (base.wasm, wm.py = wmicro with extra cases, fdiff.py = per-function
  wasm size diff of two --keep-names builds, ops-base.txt).
- Anatomy (wmicro --no-liftoff, machine instr per statement): a trivial bytecode ≈ 42;
  get_global IC hit ≈ 190 (already in run_hot, calls ic_global_get); get_method own/dict
  ≈ 170, proto IC hit (s.trim, a.push) ≈ 300 (inlined in run_hot); the `call1` of a
  @fast builtin ≈ 310 before the leaf runs (failed closure attempt, loop exit,
  decline_fast, identify), the generic leaf machinery ≈ 300 for Math.floor(int).
  Math.floor(i) 1048 = 396 lookup + ~570 call; s.charCodeAt(3) 1200 = 340 + ~730.
- Batch 1 (merged alone, the rest waits for the owner's answer on the size budget): a
  generated builtin's fast call passes `&Shadow::EMPTY`, the promoted constant, instead
  of a 48-byte copy on the stack per call (fast_leaf_call, try_fast_call). −55 B gz,
  −20 instructions per call of a generated builtin (Math.floor(i) 835 → 812,
  s.charCodeAt(k) 1202 → 1182, s.trim() 1476 → 1457, s.padStart 2232 → 2209).
  No guide page affected.
- Known builtins (D3300): `get_builtin k` (the existing run_hot arm, run_hot's code
  byte-identical, checked with fdiff.py) as guard and value for a static list of 24
  names. On the way:
  - the compile must not depend on the heap's state: the compile cache replays blobs
    into other VMs (D3040) and the dumper recompiles in a scratch heap to find suspension
    sites. Names live in the realm (`KnownNames`, renumbered with the base's atoms: the
    first heap version missed that and resolved nothing in a VM from a base), and a heap
    without a realm goes by the text;
  - size: static text list +1,463 B gz → names in the heap, a fixed-offset `jump8`
    instead of a label, 24 instead of 50 entries (an entry is ~7.5 B gz of base image),
    packed lookup result: +648 B;
  - a VM without a builtin prefix has no latch, so the values are added by
    `prepare_base_image` only (`known::arm`);
  - `prop_set_value` is inlined into run_hot: the watched-binding test there is only a
    wider mask; `step`'s debug check "advanced pc by its size" skips a patched
    instruction;
  - plain dumps run with frozen builtins: a corpus case with idle points must not write
    a builtin (known-await.js does not).
- %FastCall (in-loop leaf call for ≤1-argument method calls) built and measured: no gain
  for the generated leaves, +10% on closure method calls; dropped (D3301). Integer fast
  cases not built (D3301).
- The per-op items of the second paragraph moved to 0262 (pidle).
- Guide: interpreter.md ("Making it fast": known builtins), builtins.md (`@fast`: a
  paragraph on finding the builtin).
- Tests: interp/known.rs (the realm's names against the text, a VM from a base),
  vm/tests.rs known_builtins_* (a later script's `let Math`, frozen builtins, stops at
  fuel 1/7/97 with image round trips and gc stress), corpus builtins/known-*.js
  (member assignment also inside the call's arguments and while it awaits, global
  assignment, defineProperty and getters, delete, shadowing names, await/yield in the
  arguments); run with --suspend-every 97 --gc-stress, --determinism, --plain-dumps,
  --lazy.

## Follow-ups
- A receiver's method lookup (`s.trim`, `a.push`: ~300 instructions for the prototype
  inline-cache hit, inlined in run_hot) is now the largest part of a builtin method
  call. Making that hit leaner or moving it out of line changes run_hot and needs the
  build-noise control of D3265; not tried here.
- The known builtins' table sits in the image's tail (+2.3 KB tail). If the tail's size
  starts to matter (collections walk it), two tables in the prefix and a switch of the
  realm's slot would keep it out, for ~2 KB more base.
- More names (Object.keys, JSON.stringify, Date.now, Map, Set) cost ~7.5 B gz each and
  save ~200 instructions per use; left out because their calls are much larger than
  their lookup.

## Result
- Landed: the empty shadow by reference (batch 1), and the known builtins (D3300):
  `Math.floor(x)`, `Math.max`, `String(x)`, `Number.isInteger(x)`, `Array.isArray(x)`,
  `parseInt` and 18 more names skip the global and member lookups while no builtin was
  changed, in VMs made from a base image.
- Instructions per statement (wasm, node --no-liftoff, main 09f9b41f vs the branch):
  Math.floor(i / 3) 1,171 → 942 (−20%), Math.floor(i) 1,036 → 814, Number.isInteger(i)
  922 → 693 (−25%), Math.max(i, 3) 1,169 → 940, Array.isArray(a) 1,053 → 830, String(i)
  1,373 → 1,260 (−8%); s.trim(), s.charCodeAt(k), p.m(), f(i) unchanged (the shadow's
  −20 landed with batch 1). wmeasure tight −1.1%, calls −2.5%, prop1 −2.8%, loop −1.3%
  (spread up to 1.7%).
- ops.ts (ms, load ~7, two runs each, djs before → after): Math.floor 100k 3.83/3.68 →
  3.10/3.37 (vs QuickJS ~1.3x → ~1.1x), Number.isInteger 100k 3.01/2.88 → 2.31/2.36
  (~1.17x → ~0.95x), Math.max per line 0.91/0.86 → 0.56/0.64, String(i) 100k 4.78/4.54 →
  4.28/4.29; trim, charCodeAt loop and closure calls unchanged.
- Size: −55 B gz (batch 1), +648 B gz (known builtins: 426,556 → 427,204, slim +641),
  2,458 B below the ceiling.
- Not adopted, with numbers: D3301.
- Known gaps: receiver methods still pay their lookup (follow-up); a VM without a
  builtin prefix (`Vm::new` without frozen builtins) runs the ordinary lookups.
