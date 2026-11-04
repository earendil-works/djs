# 0183: The gate in seconds

status: done
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

- Baseline, the old gate in a lab worktree (`.worktrees/gate-lab`, a clone of the gate's
  target; one trivial commit per category, `nice 5`, machine load 10–16 from other agents):
  docs-only 249 s, js-only 252 s, a core change (comment in compiler/mod.rs) 407 s, a heap
  change (comment in heap/cells.rs) 581 s. Real gates on Nov 3: 239–573 s.
- Where the time went: everything serial; `cargo test` ran its ~40 binaries one by one
  (djs_core's lib alone ≥33 s: `vm::tests::hot_loop_is_invisible` ran 16 traces in a row);
  the difftest leg was one case (array-from-arraylike under GC stress, 77 s); every `-p`
  cargo invocation rebuilt djs-core in another feature set, one after another.
- difftest: stress variants are tasks of their own, longest first by `--times` (D2667);
  array-from-arraylike builds 10000 elements (the 100k build is quadratic under GC stress,
  which copies the growing array at every collection: expected for GC stress, not an
  engine bug); hot_loop_is_invisible on threads (33 s → ~9 s). The main difftest leg is
  now ~5–20 s.
- scripts/gate.py (D2660–D2664): legs and builds, a target directory per cargo
  configuration (all build at once), legs run binaries directly, test binaries in
  parallel, fail fast; each leg cached by its input class's tree hash plus outside inputs;
  `paths` guard; `--full` with full-only legs (example's pi-dump-every-idle.sh).
  Measured (lab, load 10–20): docs-only 7–8 s, js-only 12 s, core 149–163 s, heap 137–211 s.
- macOS: every new executable's first exec waits for syspolicyd (an online notarization
  check per binary, ~180 ms each, serialized: 351 in 10 minutes during these runs; test
  binaries sat in state S for 30–80 s). Running each new binary once while the build goes
  on (D2665) took the test leg from 84–125 s to 13 s, but the build waits instead; under
  a bad syspolicyd spell a single doctest exec waited minutes. Owner action proposed:
  Developer Tools permission for the app running agents and the gate.
- Builds off the longest path (all but the workspace test build) run at nice 4.
- Part 1 merged (fbb35ee1..ce8a6148); its own gate, with every new target directory
  cold: 223 s.
- Part 2, the protocol (D2666): merge-to-main gates under a gate lock and takes the merge
  lock only to fast-forward; main moved → rebase and gate again (cached); third attempt
  holds the merge lock throughout; locks always gate-first. It runs a copy of itself (the
  rebases can rewrite the file bash is reading). Tested in a scratch repo
  (/tmp/mtm-test): three concurrent merges plus coordinator commits, and a main that moves
  every 1.5 s (third attempt locked, merged). gate.py: flock per worktree, and a pass
  counts only if HEAD stayed put, so an old-protocol merge-to-main that checks out under a
  new gate (or the reverse) fails safely.
- Part 2 merged (..f9a0a6b6); its merge: 3 attempts (main moved twice), the last 7 s with
  20 legs cached, merge lock held 17 s in all.
- Part 3: clippy legs for the shipped feature combinations (D2668, after slim+dump broke
  main unnoticed); `--full` in the primary checkout runs in `.worktrees/full` (D2669);
  leg names without commas (`--only` splits on them); djs-test262's four integration
  tests in one binary.
- Final numbers, same lab method (load 20–40 during the run): docs-only 8 s, js-only
  12 s, core 127 s, heap 139 s. Core/heap: the workspace test build is the longest path
  (~110 s under that load, ~35 s alone), much of it waiting for macOS's first-run checks.
- Guide: docs/guide/testing.md's gate section rewritten (legs, parallel builds, cache,
  classes, full run). TESTING.md "The gate" rewritten; AGENTS.md: the cache note under
  "Testing while iterating", the coordinator's daily full run.

## Follow-ups

- Owner: Developer Tools permission for the app that runs agents and gates (System
  Settings › Privacy & Security › Developer Tools). Every new executable's first run
  waits for syspolicyd's online check today; it is most of a Rust change's gate.
- A second gate worktree, if the gate lock becomes the queue (D2666 keeps one).
- Fewer test binaries elsewhere: djs-bgen's three and djs-cli's two integration tests
  could be one binary each (docs/cli.md and continuation-dumps.md name the files);
  bins without unit tests could set `test = false` (five empty 1 MB harnesses), with a
  guard so a later `#[test]` there is not silently skipped.
- The doctests leg compiles and runs a fresh merged doctest binary per crate (3 doctests,
  all ignored) on every Rust change.
- Unifying the cargo configurations (test262, difftest and the docs each build djs-core
  in another feature set) would save CPU but changes what is tested; owner's call.

## Result

The gate is scripts/gate.py (gate.sh execs it; TESTING.md "The gate"): legs and builds,
every cargo configuration in its own target directory so all build at once, legs cached
by input class (all / nodocs / rust) plus outside inputs, an always-run `paths` guard,
`--full` (no cache, plus full-only legs; daily on main, in .worktrees/full), `--list`,
`--only`, `--no-cache`, `--clear-cache`. merge-to-main gates outside the merge lock and
holds it only to fast-forward, re-gating (mostly cached) when main moved (D2666).
Before/after (lab, trivial commit per category, loaded machine): docs-only 249 → 8 s,
js-only 252 → 12 s, core 407 → 127 s, heap 581 → 139 s; merge lock held seconds instead
of the whole gate. Decisions D2660–D2669. Known gaps: see Follow-ups (macOS first-run
checks dominate Rust changes; one gate worktree).
