# 0275: Regexp execution with captures and global replace; JSON.parse of small objects

status: in-progress
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

## Follow-ups

## Result
