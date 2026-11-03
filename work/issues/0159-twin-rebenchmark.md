# 0159: Re-benchmark every @native / @fast twin; keep only those that pay

status: done
agent: perf
branch: issue/0159-twins
depends-on: 0137, 0154, 0155 (instr.py)
guide: docs/guide/builtins.md
touches: crates/djs-core/src/builtins/ (@native/@fast annotations, hand-written fast entries), crates/djs-bgen/ (only if needed), tests/scores/size.tsv

## Goal
docs/architecture-bloat-audit.md step 3: `@native` builtins are −10.6 KB gz and `@fast`
entries −8.3 KB gz of module code. Re-measure each twin on the workloads, pi-like, the
pi bench and the 310 codemode corpus cases (wasm instructions; instr.py's builtin split,
D2309) and drop those that don't pay their bytes, with an explicit rule (e.g. keep if it
saves >= X% on a workload or >= Y M instructions per KB). Keep run_hot. Record the rule and
the per-twin table as a decision; lower the size ratchet.

## Log
- Measurement build (work/scratch/0159/twinmask.py, never merged): a byte mask in the
  module (`DJS_TWIN_MASK`, patched per variant in the .wasm, so every variant has the same
  code) turns single `@fast` entries / `@native` builtins off; per-twin call counters
  (native env + `djs_twin_hits_ptr` in wasm, summed over the pi bench's instances by
  pihits.ts).
- Calls per twin over the 22 workloads, the pi bench and the 310 corpus cases
  (work/scratch/0159/hits/). The corpus's calls of any twin are small (≤ 2.3k);
  whole-corpus native instruction counts vary ±4% run to run, so the corpus decides
  nothing.
- Per-twin wasm instructions (measure-default.py, results-default.tsv): first pass with
  `--no-liftoff` (less noise) missed costs under default tiering (indexOf off: strings
  +5% only with Liftoff); the decision uses default-tiering numbers.
- Rule (D2310): keep if ≥ 1% on pi-like / pi bench, or ≥ 10%/KB gz on a workload; sort
  pair and JSON serializer natives kept as groups (coordinator). The combined drop set
  cost pi-like +1.5% / pi bench +1.7%; keeping back ToPropertyKey, ToString and join
  (198 B gz) brought it to +0.40% / +0.32% (coordinator chose the trimmed set).
- Real build vs main: 455,081 → 448,823 B gz (−6,258); workloads −2.2..+0.92%, pi-like
  +0.45%, pi bench +0.1%, async +0.65%.
- Rebased on 0160 (slim): gen/ (incl. prelude-slim.js) regenerated, registry checked
  (slim list too); full 450,369 → 445,107, slim 385,842 → 379,276; both ceilings
  lowered with budget.py --update [--slim].
- Tests: lib, e2e (all modes), difftest native + bytecode ratchets, test262 String/Array/
  JSON/Math/Number/Promise/Object.hasOwn/parseInt/parseFloat, workspace tests.
- Guide: builtins.md (counts, the rule).
- Shared files: builtins/*.ts tag removals announced through the coordinator (slim:
  no conflict).

## Follow-ups
- The 10%/KB rule's size estimate is per function; helpers shared by kept and dropped
  entries were attributed to neither.
- docs/architecture-bloat-audit.md's step 3 numbers (−10.6 / −8.3 KB) predate this.

## Result
54 `@fast` entries and 2 `@native` builtins dropped by the rule in D2310 (per-twin table
there); 18 entries and 8 natives stay. After rebasing on slim (0160): full module
450,369 → 445,107 B gz (−5,262), slim 385,842 → 379,276 (−6,566); ceilings 446,131 and
380,300; no
workload above +0.92%, pi-like +0.45%, pi bench within noise.
