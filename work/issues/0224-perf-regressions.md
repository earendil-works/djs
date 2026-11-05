# 0224: Investigate and fix the native performance regressions since 2026-11-01

status: done
agent: perfreg
branch: issue/0224-perf-regressions
depends-on: 0223
guide: pages quoting the affected numbers (generated via status.tsv)
touches: whatever the bisection points at (regexp compiler/matcher, gc, wire/stringify, wasm instantiate); scripts/bench.sh; tests/scores/bench.tsv

## Goal (owner: investigate hard and fix)
0223 re-measured the engine benchmarks (tests/scores/bench.tsv, machine Mac17,7) against the
2026-11-01 recording and found:
- confirmed by a second run: `regexp/compile` +26-83%, `gc` per MiB of live data +60-66%,
  `wire/stringify 10k-objects` +48%;
- one run only: UTF-16 literal regexp scans +14-27% (Latin-1 unchanged), `wasm/instantiate`
  29 -> 68 µs (module ~2x larger), `engine/djs/empty` 0.005 -> 0.131 ms (maybe measures something
  else now).

For each: confirm on a quiet machine (load < ~4), bisect to the commit(s) (git bisect with
`scripts/bench.sh <filter>` in a scratch worktree; many commits since 2026-11-01, so bisect, don't
guess), find the root cause, fix it (or justify a deliberate trade with numbers and a decision),
and add a guard where cheap (a bench threshold or an instruction-count test) so it can't silently
come back. Re-record with `scripts/bench.sh --record` at the end and update status.tsv
(scripts/status.py bench). Also check whether the wasm (shipped module) numbers moved the same
way. Write findings to work/research/0224/README.md.

Constraints: another agent (0225) works on binary size at the same time; tell the coordinator
before restructuring code it may also touch. Batched merges, gate checks locally first.

## Log

- 2026-11-05: confirmed at 23b0a946 (load 3.5–8): regexp/compile, gc per MiB, wire/stringify
  and engine/djs/empty as 0223 reported; UTF-16 scans and wasm/instantiate needed a closer
  look. Sampled 16 commits since 0394842cf4, then `git bisect run` per regression on a
  bench threshold (work/scratch/0224/bisect*.sh); instruction counts from a fixed-work
  harness (`/usr/bin/time -l`) wherever timings were noisy. Findings and numbers:
  work/research/0224/README.md.
- regexp/compile: e354b3664d (STAR follow sets computed for every concatenation element).
  Fixed in regexp/compiler.rs + charset.rs, identical bytecode; guard tests/allocs.rs.
- wire/stringify: ddd792d6f2 (mark-compact): `Arena::truncate` kept a zeroed frontier, so
  strings took `alloc_from_iter`'s byte loop. One-line fix + unit test.
- gc per MiB: mostly the metric (objects 60 → 34 MiB for the same records). Per collection:
  379M instructions (2026-11-01), 296M (Cheney), 350M (HEAD). Slot plans for
  `for_each_slot`, inline mark push, single header read: 248M. Owner approved the fused
  update+slide pass: 238M (list 724M → 593M), IC emptying moved to trace with a targeted
  test. Bench reports `gc/<w> collection` now.
- UTF-16 scans: no regression in instructions; timings bimodal on both builds.
- wasm/instantiate: data segment of the compiled-prelude bench module; added
  `wasm/instantiate shipped`. engine/djs/empty: 681e323c36 runs the prelude per job;
  documented in engine.rs.
- Shipped wasm (wmeasure, base → branch): regexp-compile −13%, json-objects (stringify)
  −25%, other workloads equal or better; module +1,003 bytes.
- Checks run: djs-core lib, e2e plain/gc/suspend, difftest `--lazy --ratchet
  --suspend-every 97 --gc-stress --determinism`, native-vs-wasm, test262 RegExp +
  String.prototype.
- Guide: gc.md "Sliding in place" (three passes, update and slide in one walk).
  docs/mark-compact.md got a note. Decisions D3080–D3083.

- Merged the first batch (8193f2a6). Re-recording on main found two more: wire/stringify
  still +7% (bisected with the arena fix patched in: 5c73183db7, keys not cached in
  two-byte output; fixed, test) and startup/vm_new compiling the prelude 5.8 → 9.4 ms
  (e3093e0c6b, 0225's intrinsic_id via a UTF-8 check per name; fixed). Recorded the
  whole bench at 1d4008576c (load 3.0–4.1), status.py bench, regen.
- Merged (b1a0249a). merge-to-main had rebased 1d4008576c to 35ed5ab789 over two commits
  that add issue files only, so all rows were relabelled (sources identical). The wasm rows
  were not re-recorded: 0226 is so far an issue on main, not code, so they already describe
  the shipped module. Added crates/djs-core/tests/instructions.rs (owner): instructions of a
  `Vm::new`, ceiling +20% over today's 136M (dev); the D3091 lookup measured 198M (D3084).

## Follow-ups
- regexp compile of `/(?<y>\d{4})-.../` is still +6% instructions over 2026-11-01: the start
  sets of the counted `\d{4}` STARs. A `{n}` loop gives nothing back, so its follow
  check buys little; dropping it would change bytecode and scans, measure first.
- `wasm/instantiate` is mostly memory setup and data-segment copying: the shipped
  module's data (98 KB now) is the lever, in size work.
- Re-record the wasm group once 0226 lands the unchecked heap in the shipped module.

## Result
All six 0223 findings resolved (work/research/0224/README.md has the bisections and numbers):
- regexp/compile: follow sets only where a STAR uses them, fewer allocations; 0.44–0.71x
  0223's time, at or below 2026-11-01. Guard: crates/djs-core/tests/allocs.rs.
- wire/stringify: `Arena::truncate` moves the zeroed frontier back (mark-compact
  regression), one-byte keys cached in two-byte output; 10k-objects 3.38 → 2.37 ms. Guards:
  arena and stringify unit tests.
- gc: per MiB was mostly the metric (denser objects); per collection now 238M instructions
  (HEAD 350M, Cheney 296M) via slot plans and a fused update+slide pass. Bench reports
  `gc/<w> collection`.
- UTF-16 scans: no regression (instruction counts).
- wasm/instantiate: the bench module's data segment; `wasm/instantiate shipped` added.
- engine/djs/empty: measures the prelude since 0044; documented.
- Also fixed: 0225's intrinsic_id slowdown of prelude compilation (9.4 → 5.9 ms), guarded
  by an instruction-count test.
Shipped wasm: regexp-compile −13%, json-objects −25% instructions, the rest equal or
better. Decisions D3080–D3083. Bench re-recorded (rows at 35ed5ab789).
Guards: tests/allocs.rs (regexp compile allocations), tests/instructions.rs (prelude
compile instructions, macOS), arena and stringify unit tests, the IC-cache test, and the
bench's per-collection gc rows.
