# 0060: Out-of-memory recovery

status: done
agent: heap
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

- Nothing was lost from the removed 0051 worktree (I had only started reading).
- Repro: infra's work/scratch/0054/oom.ts passes at every limit on current main (the
  failure is layout dependent), but a native sweep (vm/tests.rs) failed at about a third
  of the limits for every handler shape: caller catch, same-frame catch, promise
  reaction, pi's prelude pattern.
- Fix (D1600): collect when the OOM error lands in a handler (after unwinding); a
  reserve of min(cap/32, 1 MiB) below the cap, opened when an OOM is thrown and closed
  by the first collection that leaves twice the reserve free; the state is a Roots flag.
  The preallocated error already existed. Collect-at-handler alone fixed the caller
  case but not reactions or the prelude (the async body's handler is in the frame that
  holds the data); the reserve fixes those.
- D1601: the GC threshold lives in the image (Roots::GC_THRESHOLD), so the OOM point
  does not move with fuel slicing or snapshot/restore. Roots grew from 56 to 64 bytes
  (layout regenerated).
- heap/tests.rs out_of_memory_leaves_heap_valid: the cap is now 1/32 larger for the same
  contents (the reserve) and it checks that opening the reserve gives room.
- Restored the strict expectation in js/test/pi-vm.test.ts and added limits 2, 5 and 12
  MiB, with and without a catch in the script. oom.ts passes at 1-16 MiB with the fix.

## Follow-ups

## Result

An out-of-memory error is now reliably a catchable `InternalError: out of memory`, also
for handlers that allocate: the interpreter collects when the error reaches a handler,
and a reserve below the cap (min(cap/32, 1 MiB)) opens for the handlers (D1600). The
contract for JS and embedders is in D1600. The GC threshold is part of the image, so the
OOM point is deterministic across fuel slicing and snapshots (D1601). Tests: native
sweep over limits (vm/tests.rs `out_of_memory_is_a_catchable_error_at_every_limit`,
`a_script_continues_after_out_of_memory`, `the_out_of_memory_point_is_deterministic`)
and js/test/pi-vm.test.ts "the memory limit" (strict again).

Known gaps: live data can use only cap minus the reserve; a handler that allocates more
than the reserve while the data is still referenced still fails (with the same error).
