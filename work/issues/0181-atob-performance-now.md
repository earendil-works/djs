# 0181: Should djs provide atob/btoa and performance.now? (research)

status: done
agent: globals
branch: issue/0181-atob-performance
depends-on: -
guide: -
touches: docs/ (a short report section, e.g. docs/codemode-feature-usage.md addendum or a new note), work/scratch/0181/; implementation only if the coordinator approves after the report

## Owner
"Investigate if we should add atob/btoa and performance.now. I think the existing pi
codemode has access to those."

## Facts to start from
- djs already defines `atob` and `btoa` (typeof is "function"; base-families lists them);
  check they behave like the web spec (Latin-1 only, InvalidCharacterError as DOMException,
  whitespace handling, padding) against node and qjs.
- `performance` is undefined in djs. 0130's analyzer listed `performance` among
  QJS_ONLY_GLOBALS (quickjs-wasi has it). pi's host code uses performance.now() on the host
  side (packages/codemode/src/runtime/execution.ts); check what pi's *sandbox* exposes to
  scripts today (QuickJS, prelude-source.ts, the wasm host's globals).

## Questions
1. What do pi's code mode scripts see today for atob/btoa/performance (QuickJS + pi's
   prelude), and do any harvested scripts use them (work/scratch/0130, 0149 data;
   scripts/harvest/features.py)?
2. atob/btoa: conformance gaps in djs vs the spec/node; tests to add.
3. performance.now: semantics under determinism (D1680 deterministic mode: the clock moves
   only with host results; Date.now's handling), fuel, snapshots/dumps (its time origin);
   whether it should be a host-provided global (like Date's clock) or an engine builtin;
   what `performance` object members to provide (now, timeOrigin?, mark/measure no);
   cost in size; the slim build.
4. Recommendation with a small implementation plan.

## Deliverable
A short report (in the issue Result and a docs note); no engine changes until approved.

## Log
- Read pi-private (read-only): vm.ts creates quickjs-wasi 3.6.2 with Intrinsics.ALL, or
  ALL & ~WEAK_REF when deterministic. PERFORMANCE, DOM_EXCEPTION and ATOB_BTOA stay on.
  The prelude doesn't touch them. Probed quickjs-wasi from work/scratch/0181 (q.mjs,
  qdet.mjs): `performance` = {now, timeOrigin}. When deterministic, now() = clock - startedAt
  and timeOrigin = 0.
- djs: the engine has no `performance`, but js/pi/vm.ts HOST_GLOBALS has supplied
  `performance.now` since 0049 with pi's semantics (origin = startedAt when deterministic).
  It is tested in js/test/pi-vm.test.ts, and docs/host.md already says so. So pi scripts on
  djs see all three today. The issue's premise ("performance is undefined in djs") is true
  only of the bare engine.
- Harvest counts (0149/0130 extracts, counts only): atob/btoa/DOMException 0;
  performance.now 3 distinct scripts (all ok on quickjs; elapsed-time reporting);
  timeOrigin 0.
- Conformance: work/scratch/0181/b64.js (46 cases) on node, quickjs-wasi, dquickts qjs and
  djs. djs = qjs on every line, and = node on every data/error case. Differences are only
  missing-argument handling (quickjs-wasi treats it as undefined), enumerability (node
  true), and `new atob` (node constructs). Added it as the difftest case
  tests/corpus/builtins/base64-edges.js (.expected from qjs). It passes on djs plain,
  --suspend-every 97 and --gc-stress.
- Guide: no pages affected.

## Result
Report: docs/web-globals.md. pi's code mode scripts see `atob`, `btoa` and `DOMException`
(quickjs-wasi intrinsics) and `performance` = {now, timeOrigin}. 3 of 2,687 distinct
harvested scripts use `performance.now`, and none use atob/btoa/timeOrigin. djs already
provides all of it: atob/btoa/DOMException as native builtins that match the spec and
qjs, and `performance.now` from the pi adapter with pi's deterministic semantics. The only
gap is `performance.timeOrigin`.

Recommendation: no engine change. An engine `performance.now` could only read the
host-set Date clock minus an origin. It would add a realm slot, a setter and about 1 KB to
every host, and buy nothing over the adapter's three lines. Determinism, fuel and snapshots
are already right in the adapter, because the origin is closure heap data and the clock
moves only with results. Optional adapter-only follow-up: `performance.timeOrigin` (0 when
deterministic, like pi's quickjs; otherwise the run start), one line plus a pi-vm test.
Landed: the report and a base64 edge-case difftest case. No engine changes.
