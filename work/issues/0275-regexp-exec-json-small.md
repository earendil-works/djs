# 0275: Regexp execution with captures and global replace; JSON.parse of small objects

status: done
agent: pscan
branch: issue/0275-regexp-exec-json-small
depends-on: -
touches: crates/djs-core/src/regexp (matcher/exec, match-result building), crates/djs-core/src/builtins (RegExp.prototype.exec/match/replace fast paths, their generated/native parts), crates/djs-core/src/json/parse.rs (object building), swar.rs, tests/corpus
guide: builtins.md if a fast-path story changes
decisions: D3460-D3469

## Goal
pidle's 0270 sweep (wasm instr per op incl. loop overhead, vs QuickJS 2026, main ~9ab27ba5;
re-measure on current main first):
- `LINE.match(/^([^:]+):(\d+):/)` 37.9k vs 11.6k (3.3x): a non-global match with captures,
  the shape of every grep-output parser in codemode.
- `S.replace(/\s+/g, ' ')` 40.4k vs 16.3k (2.5x).
- `JSON.parse` of a 5-key object 21.8k vs 16.0k (1.36x): small-object parse cost (shape
  creation, property definition), the shape of every tool result.
Profile each: matcher work vs the result array (index, input, groups, the captures as
strings), lastIndex handling, the generic exec/replace protocol vs a leaf; for JSON, the
per-object and per-key costs. Fix what's cheap.

## Acceptance criteria
- [ ] per-op instructions vs QuickJS before/after; text/heavy/loop/json workload instructions
- [ ] exact semantics (test262 built-ins/RegExp, String.prototype.match/replace, JSON; corpus with --suspend-every 97 --gc-stress)
- [ ] size reported: room is ~100 B under the owner's D020 budget unless pcall gives bytes back. Prefer size-neutral changes; ask before needing more
- [ ] gate green; decisions recorded

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
- Re-measured on main 207945d6 (slope.py, wasm instr per iteration incl. ~300-500 loop;
  QuickJS = quickjsBuilt bellard via qb-run.ts, quickjs-wasi via qjs-wasm-run.mjs):
  match groups djs 38.0k / QuickJS 12.8k / quickjs-wasi 19.2k; replace /\s+/g 39.1k /
  16.4k / 73.0k; JSON.parse 5 keys 22.3k / 18.9k / 22.9k; regex test 10.1k / 11.5k / 14.0k.
- Profile (native samply, instr build): match = 130 builtin bytecodes (String.prototype.match
  23, GetMethod 16, @@match 28, flags getter 13, ThisRegExpObject 2x6, RegExpExec 11) and
  regexp_pristine 16% inclusive (a dozen prototype lookups + own_keys Vec per call).
- D3460: regexp_pristine answers from the builtins latch; sole_property instead of own_keys;
  String.prototype.match/replace call the leaves for a pristine RegExp. match −45%, replace −34%,
  +119 B gz (approved).
- D3461: ExecState memory reuse measured (−6% match, −8% test) but +100-118 B: not landed.
- D3462: create_data_property quick insert for arrays (match result's index/input/groups): −9%
  match, +10 B.
- D3463: JSON parser Vec capacities via buf::vec_with_capacity: −4..−9%, −30 B.
- Remaining: the matcher itself (`\s+` replace: resume_in 30% native), match_result's
  substrings, the per-literal RegExp object (create + lastIndex define 4%), key atomization in
  JSON (keyed hash + atom table lookups per key, the key cache is per parse).
- Corpus case tests/corpus/builtins/regexp-pristine-paths.js (own @@match/exec, subclass,
  String object, non-string replacement, patched RegExp.prototype.exec and flags).
- Guide: none affected (builtins.md does not describe the RegExp fast paths).

## Follow-ups
- The matcher's per-match memory (D3461, −6% match / −8% test) if room appears; patch in
  work/scratch/0275/re-state-reuse.patch.
- Regexp literal per evaluation: `create` defines lastIndex through define_own_property (a
  quick insert or a cached shape would save ~4% of a match).
- `\s+`-style replace: resume_in/Machine::run is ~30% (native) of the remaining 26k; the
  matcher's per-match setup (exec_in reserve, find) is the next lever.
- JSON.parse key atomization: the key cache is per parse, so a one-object parse hashes every
  key (keyed hash + base/session atom lookups); a VM-lifetime cache would cost heap room.

## Result
Landed (D3460, D3462, D3463; D3461 measured, not landed). Wasm instructions per iteration
(slope.py, incl. the loop; main 207945d6 → branch; QuickJS built / quickjs-wasi):
- `LINE.match(/^([^:]+):(\d+):/)`: 37.7k → 18.4-18.9k (−50%); 12.8k / 19.2k (was 2.95x
  QuickJS, now 1.45x; below quickjs-wasi).
- `S.replace(/\s+/g, ' ')`: 39.1k → 26.0-26.5k (−32..−35%); 16.4k / 73.0k.
- `JSON.parse` of a 5-key object: 22.3k → 18.7-20.5k (−4..−9%, noisy); 18.9k / 22.9k.
- `/TODO|FIXME/.test(LINE)`: unchanged (~10k; 11.5k / 14.0k).
Workloads (wasm instructions per run, ins.py, main → branch): heavy −1.8%, text −3.8%, json
−1.6%, loop and inspect unchanged.
Size: +113 B gz on the default module; after the rebase onto 0270 and 0277 both modules fit
their ceilings unchanged (429,585 of 429,648; slim 363,555 of 363,609).
Tests: test262 built-ins/RegExp, String, JSON, Array, Object, Reflect, Proxy and
language/expressions/object, language/literals all pass; difftest builtins, codemode,
codemode-harvest, language plain, --suspend-every 97 --gc-stress and --plain-dumps; new corpus
case builtins/regexp-pristine-paths.js.
