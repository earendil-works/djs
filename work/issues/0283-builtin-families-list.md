# 0283: What each builtin family costs and gives: a list for the owner's decision

status: done
agent: pstart
branch: issue/0283-families-list
depends-on: 0271 (research)
touches: work/research/0283/ (the list and its scripts), nothing in the engine
guide: -
decisions: -

## Goal
The owner asked (2026-11-21) for a list of what leaving each builtin family out of the
shipped module gives and costs, to decide. Write work/research/0283/families.md: one row
per family (typed arrays/ArrayBuffer/DataView, Iterator helpers, Proxy, Reflect,
DisposableStack/SuppressedError/using, decorators, web globals (which ones), WeakMap/WeakSet,
WeakRef/FinalizationRegistry, Array.fromAsync, BigInt, and anything else separable:
Intl-like bits, generators? Symbol extras?) with measured columns:
- base image KB, and the per-VM memory effect (does it move a page, alone and in the
  combinations that cross one: say which combination reaches idle 320 KiB);
- shipped module bytes saved (raw and gzip; the family's builtins, prelude code, and any
  Rust that becomes dead);
- startup effect (new VM);
- usage: in the 2,535 real codemode scripts (docs/codemode-feature-usage.md, 0130) and in
  pi's prelude/lockdown (what breaks in pi if it goes; what a stub would need);
- conformance: test262 tests that would then fail (count per family);
- what a script sees if it uses it (ReferenceError on the global, missing methods);
- the lazy-chunk alternative's cost, where it applies.
End with 2-3 recommended packages (e.g. "pi-safe minimum", "idle-320", "max size") with
their totals. Keep it short and readable: the owner decides from it.

## Acceptance criteria
- [x] families.md with measured numbers (state the method and load)
- [x] merged as docs (work/research), gate green

## Log
- Measured at 7b302395, load 20-80 (others building): sizes, pages and instruction counts
  only. Scripts in work/research/0283 (they write to work/scratch/0283).
- Base per family (famvar.sh), module/idle/rows per family and per package (famrun.sh),
  Rust by function sizes plus the full-vs-slim build delta, test262 by t262.py over the
  active paths, startup by icount.sh.

## Result
work/research/0283/families.md: per family base KB, module bytes, test262 tests, usage, pi's
needs, what a script sees and the lazy alternative; three packages. Idle 320 KiB needs
package B (slim set + fromAsync + URI + DOMException/atob/btoa, Int8Array stub): idle arena
~127.0 KB, five codemode rows -64 KiB, fanout +64 (growth step), module ~-35 KB gz, 4,864
test262 tests. Proxy and Reflect can't go without breaking pi.
