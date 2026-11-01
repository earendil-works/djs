# 0111: Prototype objects' property order (qjs)

status: done
agent: infra
branch: issue/0111-prototype-order
depends-on: 0108
guide: -
touches: crates/djs-bgen/src/qjs_order.rs, builtins/ (install order), tests/corpus/

## Goal
0108's follow-up: prototype objects' own property order differs from qjs (qjs puts
`constructor` last and uses its own method order). Extend D2072's qjs-order table to
prototypes (Array.prototype, String.prototype, Object.prototype, Promise.prototype,
Map/Set, RegExp, Date, TypedArray, Error, …).

## Acceptance criteria
- [x] Prototype property order matches qjs for every builtin prototype (a difftest case
      listing getOwnPropertyNames/Symbols of each), test262 unchanged, both backends.

## Log
- Compared Reflect.ownKeys of every builtin prototype (and the %…Prototype% intrinsics)
  with qjs.
- D2150:
  - `constructor` became a bgen-emitted install (`%InstallPrototypeConstructor`), placed
    last by default and by the table where qjs differs;
  - Error.prototype's name/message moved to `@value` installs;
  - the function-kind prototype links are made at constructor install;
  - the table entries were generated from qjs's output.
- Now every prototype matches qjs except keys djs lacks (Annex B, qjs extensions).
- Corpus case builtins/prototype-property-order.js (those keys filtered on both sides),
  identical on both backends and in the stress modes. bgen golden updated.
- Checked:
  - djs-core lib, e2e plain, djs-cli and djs-bgen tests;
  - the corpus ratchet;
  - test262 built-ins plus class/generator dirs (native) and the full active group on the
    bytecode backend (40 expected, no new failures).
- Guide: none affected.

## Follow-ups
- Annex B String.prototype HTML methods, Date.prototype.setYear,
  RegExp.prototype.compile and the `__defineGetter__` family exist in qjs but not in djs
  (Annex B is skipped in test262; models rarely use them).

## Result
Every builtin prototype's own property order matches qjs, apart from keys djs does not
have (D2150).
