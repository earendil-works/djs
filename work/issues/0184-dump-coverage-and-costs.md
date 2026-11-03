# 0184: Close the remaining dump refusals; measure dump size and load time

status: done
agent: portable
branch: issue/0184-dump-coverage
depends-on: 0182
guide: -
touches: crates/djs-core/src/compiler/ (suspension maps for the unsupported constructs), crates/djs-core/src/vm/dump*.rs, tests/dump/v2/ (new golden programs under D2619), tests/smap pins, scripts/snapshot-research or scripts/dump-costs (measurements), docs/continuation-dumps.md

## Goal
Dumps are the persistence format (D2560); before images are retired (0177):
1. **Remaining refusals** — describe what a suspended frame holds at, and dump/resume:
   `switch` case tests (the discriminant twice on the stack), `finally` blocks (completion
   records), `using`/`await using` regions, destructuring, `yield*` (the delegated iterator),
   closing an async iterator, `for-in` (the key iterator), RegExp string iterators, iterator
   helpers if feasible. Each: the map roles, the loader's resume side, re-enactment, golden
   programs, every-idle-point tests. Refuse cleanly whatever stays out.
2. **Costs vs images** (the owner's comparison): dump size (raw and zstd) and write/load time
   (wasm, the dump module) next to session images/containers for: an empty Sandbox, typical
   pi waiting sessions (scripts/snapshot-research cases, the harvested corpus idle points:
   distribution), and data-heavy sessions (records, a 5 MB log, binary buffers). Recommend
   whether anything (e.g. zstd of dump text, slices/shared strings in dumps) is needed before
   0177 retires images.

## Log
- 2026-11-04 (portable): costs measured with scripts/dump-costs/dump-costs.ts (wasm
  profile builds; report work/scratch/dump-costs/report.md; interim note sent to the
  coordinator with the full tables). Corpus (300 cases, 1,063 idle points, packaging B)
  p50/p90/p99/max: dump raw 6.1/13.6/165/165 KB, zstd-3 1.7/3.1/17/17 KB, container
  5.1/9.7/34.5/34.6 KB, image 23/41/218/221 KB; dump 3.5/6.0/43/69 ms, resume
  1.3/2.3/13/29 ms, bytes() 0.7/1.0/5.8/15 ms, fromBytes 0.3/0.5/2.3/14 ms. Data-heavy:
  records dump 867 KB -> 236 zstd vs container 372; big-log 1,816 -> 394 vs 416; log-5mb
  3,749 -> 924 vs 930; binary 2,493 -> 1,570 vs 1,446 (typed arrays as decimal
  elements). Fixed floor ~2-3 ms per dump in wasm (0.8 ms native), 90% of it
  registry::reachable_names per dump. Recommendations: hosts zstd-3 dump text; typed
  arrays as base64 bytes; cache the builtin names per base; no shared strings needed.
  Only refusal in the cases: yield* (agent-loop, 3 of 11 points).
- Coordinator approved all four. (2) turned out done already: typed arrays over 64
  elements were `new Float64Array($dump.bytes(...))` before (checked the binary case's
  dump); the binary case's 1.3x raw / 1.09x zstd is base64's cost. (3) D2626: the walk
  skips seen cells before making paths (805 -> 433 us native, same 734 names) and the VM
  keeps the table after its first dump (next dumps of an empty VM 0.05 ms native).
  Re-measured (report-after-names.md): A dump ms median empty 2.77 -> 0.93, ask 1.96 ->
  1.17, sequential 2.13 -> 0.74; B sequential 2.40 -> 1.80, corpus p50/p90 dump 3.50/5.98
  -> 1.84/3.65 ms (B makes a fresh dump-module VM per dump: only the faster walk). Docs:
  hosts compress dump text (continuation-dumps.md).
- yield*: `Loop::Star` (roles `yield*.iterator`, `yield*.next`, written in `loops` with
  kind "yield*"), SiteKind::YieldStarAwait (`yield*.await#N`) split from the yields
  (`yield*#N`); a sync generator at `yield_star` is SUSPENDED_RAW (exporter accepts it,
  loader sets it); fromSync handling extended to Loop::Star. Golden yield-star.src.js
  (10 dumps), test delegation_round_trips (every idle point + chained). Smap pins
  re-pinned (generators.smap), smap_tests CONSTRUCTS gained yield* cases.
- Closing an async iterator: emit_async_close_normal(site, hide: Option<usize>): break
  and yield*'s no-throw close describe their stack (loop roles hidden); the close
  subroutine path (`v ret it`, a return address) stays unsupported. A throw out of a
  for await body: `Role::Exception` below the close's await. Corpus pins: no "closing
  an async iterator" left; unsupported now 19 using + 1 destructuring.
- for-await-close golden program (work/scratch/0184/for-await-close.src.js) waits for
  finally: its awaits sit in an async generator's finally block.

- finally (D2627): roles `finally.value` / `finally.return(i)` (the i-th subroutine of the
  function, recorded by `Parser::subroutine`); resolve.rs records each `gosub`'s position
  and the map gives every site its subroutines' return pcs (`Site::returns`); dumps write
  the address as the gosub's index, the loader maps it back. The for-await close
  subroutine now consumes the iterator before its await (`v ret`): same roles. Case tests:
  `switch.discriminant` twice. Promise value thunks / throwers in flight (kind 9/10: the
  AsyncIteratorCloseThrow `ignore` handler, finally's thunks) written as arrows: a throw
  out of a for await over an async generator whose finally awaits refused before.
  Goldens finally (15 dumps) and for-await-close (3); both in delegation_round_trips.

- using (D2628): `using.record` under the statements; the await-using disposal's site
  `using.record finally.value finally.return using.record`; `$dump.using(resources,
  state)` written once per Record (exporter marks the node from the role). Golden using
  (11 dumps).
- Destructuring (D2628): `pattern.source/excluded/key/next/iterator`, targets' assignment
  roles, `assign.value` for an assignment pattern expression. Corpus pins: no unsupported
  site left (summary.txt). Golden destructuring (10).
- for-in (D2628): `$dump.forIn(o, keys left)` in `loops` (`{kind: "for-in", iterator}`);
  `given()` now only takes 1-slot Records (cells must be BoxCells). Proxy for-in refused.
  Golden for-in (8).
- RegExp String Iterators (`$dump.regexpIterator`) and iterator helpers
  (`$dump.iteratorHelper(slots)`). Golden iterator-helpers (10). Found and fixed: an array
  with named properties held once by a frame was written inline with location-less
  patches (`simple()` let arrays with named keys through).
- Kept refused: sites inside a class definition (class built from text), closing a
  for-await iterator on a resumed generator's `return(v)` path. Docs:
  continuation-dumps.md (primitives table, statements in flight, still refused),
  functions-as-code.md refusal table. Guide: not affected (no page covers dumps' roles).
- Note: `cargo clippy -p djs-core --no-default-features --features dump` fails on main
  too (`base64` unused in slim); not in the gate.

- Final dump-costs run (report work/scratch/0184/report-3.md, machine load 11-27, so
  timings are noisy): corpus 300 cases, 1,063 idle points, 0 refused; agent-loop 11 idle
  points, 0 refused (3 before); sizes unchanged.

## Result
Landed (D2626, D2627, D2628):
- **Refusals closed:** `yield*` (Loop::Star, yield*.await sites), closing an async
  iterator (break, yield*'s close, a throw out of a for await: `exception`), `finally`
  (roles `finally.value`/`finally.return`, the return address written as the index of its
  `gosub`, D2627), a for-await's close subroutine (`v ret`), `switch` case tests
  (`switch.discriminant`), `using`/`await using` (`using.record`, `$dump.using`),
  destructuring (`pattern.*`, `assign.value`), for-in (`$dump.forIn`), RegExp String
  Iterators (`$dump.regexpIterator`), iterator helpers (`$dump.iteratorHelper`); promise
  value thunks/throwers in flight as arrows. Fixed: an array with named properties held
  once by a frame (patches without a location).
- **Still refused (cleanly, by path):** a site inside a class definition, closing a
  for-await iterator on a resumed generator's `return(v)` path, a for-in over a Proxy.
- **Tests:** golden programs yield-star, for-await-close, finally, using, destructuring,
  for-in, iterator-helpers (tests/dump/v2/golden/, 64 dumps) with every idle point
  dumped once and chained (`delegation_round_trips`); smap_tests tables STATEMENTS,
  USING, PATTERNS; loader refusal cases; pins: no unsupported site left in the corpora
  (tests/smap/summary.txt).
- **Costs** (scripts/dump-costs/dump-costs.ts; `scripts/build-wasm.sh && scripts/build-wasm.sh
  --dump`, then `scripts/timeout.sh 3600 node scripts/dump-costs/dump-costs.ts <dir>
  [--no-corpus]`). Before/after D2626 (reports work/scratch/0184/report-1.md,
  report-2.md): packaging A dump ms median empty 2.77 -> 0.93, ask 1.96 -> 1.17,
  sequential 2.13 -> 0.74; packaging B sequential 2.40 -> 1.80; corpus (B) p50/p90 dump
  3.50/5.98 -> 1.84/3.65 ms. Sizes (corpus p50/p90): dump raw 6.1/13.6 KB, zstd-3
  1.7/3.1 KB, container 5.1/9.7 KB, image 23/41 KB. Recommendations: hosts zstd-3 dump
  text (docs, done); typed arrays as base64 (already done); builtin names cached per VM
  (D2626, done); no shared strings needed. Packaging B makes a fresh dump-module VM per
  dump, so only the faster walk helps there unless js/src/sandbox.ts keeps the instance
  (example's area). Final run (report-3.md): 0 refused at all 1,063 corpus idle points.
- **Known gaps:** class-definition sites and the generator-return close (rare); slim +
  dump clippy warning (`base64` unused) predates this issue.
