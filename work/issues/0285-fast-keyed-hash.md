# 0285: A fast keyed hash instead of SipHash-1-3 (owner: no strong flood resistance needed)

status: in-progress
agent: pgc
branch: issue/0285-fast-keyed-hash
depends-on: -
touches: crates/djs-core/src/heap/string.rs (KeyedHasher, hash_latin1_with, hash_units_with, keyed_u64), heap/atom.rs, heap/collection.rs (Map/Set), json parse's key hashing callers, vm/compat.rs (COMPAT bump), docs/decisions.md
guide: heap.md / values.md where hashing is described
decisions: D3540-D3549

## Goal
The owner decided (2026-11-21): codemode doesn't need a particularly collision-resistant
hash, because a timeout bounds the whole run anyway. Record that as D3540 (owner),
superseding D2433's choice of SipHash-1-3 (keep its structure: per-VM key from the seed,
base key, caches, rekeying, determinism).
Replace SipHash-1-3 with a fast keyed hash (e.g. a seeded multiply-mix over 8-byte words,
in the family of wyhash/foldhash/rapidhash; your choice, measured; keep it keyed so
collisions can't be built offline trivially, and keep "never 0"). It is used for atom
interning, JSON keys, Map/Set keys (strings, numbers, objects, symbols) and string keys of
property access, so measure all of them:
- Map.set/Set.add with string keys (1.53x/1.41x QuickJS in the ops table), number keys (1.9x),
  Map.get/Set.has, JSON.parse (key interning), o[k], the codemode workloads, compile
  (identifier interning);
- the base image: base atoms carry the base-key hash, so the image changes (regen, COMPAT).
Keep the existing collision test meaningful (or adapt it: offline collisions of the fixed
base key are acceptable now, say so).

## Acceptance criteria
- [ ] per-op instructions vs QuickJS before/after; workloads; compile
- [ ] determinism (same seed, same heap), images, dumps; gate green; decisions recorded; guide updated
- [ ] size reported (~1 KB of the owner's allowance left, shared): a simpler hash should shrink code
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

## Result
