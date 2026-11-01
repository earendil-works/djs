# 0089: pi adapter: collect before snapshot() and before caching the prelude image

status: done
agent: snapshot
branch: issue/0089-pi-snapshot-collect
depends-on: 0088
touches: js/pi/vm.ts (+ js/pi/engine.ts if needed), js/test/, docs/host.md

## Goal
0088 found that pi's `snapshot()` and its saved post-prelude image don't collect first, so
they hold ~943 KB where ~504 KB is live, and docs/host.md's "450 KB per prelude image" is
stale. A clear bug, independent of the A/B design decision pending with the owner.

## Acceptance criteria
- [x] Collect (`vm_collect`) before `snapshot()` and before caching the prelude image; the
      image sizes before/after recorded; resumed sessions and deterministic fingerprints
      still behave identically (pi's suite and the js tests); docs/host.md updated with
      measured numbers.

## Log

- `Engine.collect()` over the existing `vm_collect` export (js/pi/engine.ts).
- js/pi/vm.ts: every cold start collects right after the prelude, cached or not (cold and
  warm starts go on from the same heap); `snapshot()` collects the session itself, then
  takes the image (continuing and resuming go on from the same heap). D1890 has the
  reasoning and the rejected alternative (collecting a copy in a scratch instance).
- A collection moves later collection points, which deterministic scripts can only observe
  through the out-of-memory point of allocations that are not retried (D1700), so
  `DETERMINISTIC_VERSION` is 2 (the fingerprint says `deterministic 2`).
- Measured with node 25 and the `wasm` profile build (work/scratch/0089/measure.ts, which
  wraps `Engine.prototype.image` to see every image; pi.bench's tools; a snapshot and a
  resume at every idle point). Load was 30–60, so times are rough:

  | image | before | after |
  | --- | ---: | ---: |
  | prelude image (cache entry) | 933.1 KB | 496.4 KB |
  | `snapshot()` waiting on the first call | 936.3 KB | 498.4 KB |
  | tool-heavy (61 results), largest snapshot | 986.9 KB | 629.3 KB |
  | tool-heavy, last snapshot | 653.1 KB | 629.3 KB |
  | `snapshot()` time | 0.05–0.10 ms | 0.37–0.41 ms |
  | `resumeVm` waiting on the first call | 1.47 ms | 0.82 ms |
  | warm start (`return 1`) | 2.7 ms | 2.0–2.4 ms |

- New test (js/test/pi-vm.test.ts): a deterministic script that makes garbage before it
  waits gives the same messages and the same number of interrupt polls with no snapshot,
  with a snapshot and the session continued, and with the snapshot resumed; its snapshot is
  within 16 KB of a trivial script's. Without the fix the size check fails (1,002,626 vs
  957,370 bytes); the other checks pass either way.
- js tests: 46 pass, 1 skipped (the compiled-prelude check needs the gate's debug build).
  pi's suite (pi c1b68f09): 221 / 226 before and after, the same five failures (the
  fingerprint prefix ×2, the sloppy-mode escape hatch ×3), report diff empty.
- docs/host.md: the prelude-image size (496 KB, was "450 KB"), the snapshot row, the
  fingerprint row.

## Follow-ups

- none

## Result

pi's adapter collects after the prelude on every cold start and in `snapshot()`, before
taking the image (D1890): prelude images 933 → 496 KB, a session waiting on its first call
936 → 498 KB, `resumeVm` about twice as fast, `snapshot()` +0.3 ms. Scripts see no
difference (same messages and interrupt polls, tested); the fingerprint's deterministic
version is 2 because later collection points, and with them out-of-memory points of
non-retried allocations, move. pi's suite and the js tests are unchanged.
