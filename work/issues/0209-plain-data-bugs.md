# 0209: Plain dump data bugs: accessors of factory closures, static private members, runtime-made objects

status: in-progress
agent: pjd
branch: issue/0209-plain-data-bugs
depends-on: -
guide: -
touches: vm/dump_flight.rs, vm/dump_plain.rs, vm/dump.rs (data/heap side only), tests/corpus, js/test/dumpload.test.ts

## Goal
Fix plain-dump bugs on the data side (found by 0208's fuzzer and by pjf in 0205):

1. An object-literal accessor that captures a factory variable is written as `get: ,`
   (the dump doesn't compile): `let n = 0; const o = { get a() { n++; return n; } };
   await tools.echo(1); return o.a;` (~60 of 300 fuzz programs).
2. Any class with a static private member is refused as "an instance of class … with
   private name #o": `class B { static #o = 0; static o() { return B.#o; } }
   globalThis.B = B;`. D2878 says static private members are unaffected, and the message
   mislabels the class as an instance.
3. 0207 follow-up: objects the dump runtime made (`$djs.fromSync`'s wrapper,
   `closeThrow`'s handler) are written as the runtime's closures (~1.6 KB per re-dump of a
   `for await` over a sync iterator). Write them as their helper call again.

Each fix gets a corpus case (or a dumpload case), and any plain-xfail marker 0208 added
for it is removed.

## Constraints
pjr is changing classes in dump.rs/dump_plain.rs (class self-reference cycles), then
dump_origin.rs; pjs is changing dump_cont.rs; pjf is deleting format 2. Keep merges small
and rebase often. Decision IDs D2950–D2959.

## Log
- Bug 1 (`get: ,`): the accessor's getter is a closure a plain dump makes in the
  continuation (`plain_patched`), so `expr` returns empty and the descriptor patch had a
  hole. `Dumper::descriptor` now reads every value with `expr_or_loc` (descriptors are
  only written in patches, after the values exist). Corpus case
  codemode/plain-accessors.js (djs and node, 2 links).

## Follow-ups

## Result
