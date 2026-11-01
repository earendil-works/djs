# 0060: Out-of-memory recovery

status: open
agent: -
branch: issue/0060-oom-recovery
depends-on: 0051
touches: crates/djs-core/src/heap/ (gc, arena), crates/djs-core/src/interp/ (the OOM throw path), crates/djs-core/src/vm/, js/test/pi-vm.test.ts (restore the strict expectation)

## Goal
infra (0054) found that OOM recovery is unreliable. Allocation never collects (only
safepoints do), so a catch handler or promise reaction that allocates right after an OOM
(pi's prelude does this) can hit a second OOM before any collection, and the OOM escapes
as a crash instead of a script error. Repro: `MB=8 node work/scratch/0054/oom.ts` in
`.worktrees/infra` (with DJS_WASM pointing at a debug djs-wasm with
`--features compiled-prelude`). It fails at 2, 4, 7, 8, 9 and 12 MB.

## Acceptance criteria
- [ ] Throwing an OOM error collects before the handler runs (a safepoint at the throw),
      and the error object itself can always be allocated (a preallocated OOM error in
      the realm, like qjs, or reserved headroom below the cap).
- [ ] A clear contract, recorded as a decision: what an OOM looks like to JS (the error type
      and message) and to the embedder, and the guarantee that a script that catches it
      and allocates little can continue.
- [ ] Native tests across many memory limits (a sweep, as in the repro) for sync code,
      catch handlers, promise reactions and the pi prelude pattern. The strict
      expectation in js/test/pi-vm.test.ts "the memory limit" is restored.
- [ ] Determinism: the OOM point for a given input and limit is deterministic (relevant to
      0055, the determinism investigation).

## Log

## Follow-ups

## Result
