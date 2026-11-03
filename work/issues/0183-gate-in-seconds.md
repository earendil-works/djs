# 0183: The gate in seconds

status: in-progress
agent: gate-speed
branch: issue/0183-gate-in-seconds
depends-on: -
guide: docs/guide/testing.md (how the gate works) if it describes the gate's steps
touches: scripts/gate.sh, scripts/merge-to-main.sh, scripts/merge-lock.sh, the gate worktree setup (.worktrees/gate), TESTING.md, AGENTS.md (the "Testing while iterating" section if the rules change), test runners (djs-difftest, djs-test262, js test runner) only for speed (parallelism, caching), docs/decisions.md

## Owner
"Fresh agent to get the gate down to seconds."

## Today
Every merge runs scripts/gate.sh serially under the merge lock. Gate wall time was ~90–130 s
on Nov 1–2 and is now 239–573 s (median ~400 s); agents then wait 2–11 min for the lock.
work/scratch/gate-steps.tsv and merge-log.tsv (primary checkout) have per-step times.
The 22:24 run: `cargo test` 153 s, difftest (ratchet + stress + images + dump at every idle
point) 78 s, unsafe-heap build 72 s, wire cross-language 23 s, clippy 21 s, slim legs ~23 s,
test262 11 s, djs-wasm 11 s, native vs wasm 10 s, guide 10 s; wall 432 s, CPU 873 s.

## Goal
Typical merges gated in **seconds** (target: under ~30 s for docs/JS/small changes, as low as
possible for core changes), **without weakening what main guarantees**. Ideas, measure each:
- **Impact-based selection**: map changed paths to the legs they can break (docs-only →
  guide check; js/ → js tests + wasm; heap/ → everything heap-related; a crate's tests only
  when that crate or its dependencies changed), with a conservative default (unknown path →
  full gate) and a periodic/explicit full run so nothing rots.
- **Parallelism**: run independent legs concurrently with the full machine (the gate is not
  niced), one shared target dir, no duplicated builds (unsafe-heap and slim builds reuse
  artifacts; build once, test many).
- **Caching**: warm incremental builds in the gate worktree (already persistent, D1792);
  avoid rebuilding release/LTO wasm when no Rust changed; cache test262/difftest results by
  input hash (engine build hash + test file) so unchanged pairs aren't rerun.
- **Faster runners**: djs-difftest / test262 at full width in release where it pays, the
  dump-every-idle leg cheaper (e.g. a sample per merge, full nightly?), cargo test with
  nextest-style parallelism if allowed by the dependency policy.
- **Merge train**: if the lock is still the bottleneck, batch queued branches (D1790
  context: it was on hold) or rebase+gate outside the lock and only fast-forward inside it
  (re-gate only if main moved in a way that affects the selected legs).
Record each choice as a decision; document the new gate in TESTING.md; keep `scripts/gate.sh
--full` for the complete run, and say how often it must run.

## Acceptance criteria
- [ ] Measured before/after per change category (docs, js, a core change, a heap change).
- [ ] No class of regression the old gate caught can reach main unnoticed (argue it per leg).
- [ ] The merge lock is held only as long as necessary.

## Log

## Result
