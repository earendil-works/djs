# 0283: What each builtin family costs and gives: a list for the owner's decision

status: open
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
- [ ] families.md with measured numbers (state the method and load)
- [ ] merged as docs (work/research), gate green

## Log

## Result
