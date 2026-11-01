# 0089: pi adapter: collect before snapshot() and before caching the prelude image

status: in-progress
agent: snapshot
branch: issue/0089-pi-snapshot-collect
depends-on: 0088
touches: js/pi/vm.ts (+ js/pi/engine.ts if needed), js/test/, docs/host.md

## Goal
0088 found that pi's `snapshot()` and its saved post-prelude image don't collect first, so
they hold ~943 KB where ~504 KB is live, and docs/host.md's "450 KB per prelude image" is
stale. A clear bug, independent of the A/B design decision pending with the owner.

## Acceptance criteria
- [ ] Collect (`vm_collect`) before `snapshot()` and before caching the prelude image; the
      image sizes before/after recorded; resumed sessions and deterministic fingerprints
      still behave identically (pi's suite and the js tests); docs/host.md updated with
      measured numbers.

## Log

## Follow-ups

## Result
