# 0038: Builtins wave 3: Map, Set, WeakMap, WeakSet, Reflect, Date; promote finished features to active

status: open
agent: -
branch: issue/0038-builtins-wave3
depends-on: 0034
touches: builtins/ (map.ts, set.ts, weak.ts, reflect.ts, date.ts …), builtins/gen/, crates/djs-core/src/builtins/, crates/djs-core/src/heap/ (new cell kinds for ordered hash tables, additive), crates/djs-core/src/bytecode/intrinsics.rs (regenerated), crates/djs-core/src/interp/realm.rs (additive), crates/djs-core/src/vm/ (only the host time hook for Date), tests/test262.toml (moving features from later to active, as listed below), ratchets

## Goal
Codemode scripts use Map, Set and Date all the time. Reflect is small and test262's
`isConstructor.js` include implies it. Plus a coordinator decision: features that are now
implemented move from `later` to `active` in tests/test262.toml, so the headline number
counts them.

## Acceptance criteria
- [ ] Map and Set (ES2025, including the Set methods `union intersection difference
      symmetricDifference isSubsetOf isSupersetOf isDisjointFrom`, and `Map.groupBy`):
      an ordered hash table cell keyed by SameValueZero (heap strings by content,
      numbers normalized: -0 → +0, NaN equal to itself), with iteration that tolerates
      deletion and insertion during iteration (spec semantics: entries added during
      iteration are visited, deleted ones skipped), and compaction. Map/Set iterators
      (`%MapIteratorPrototype%`, `%SetIteratorPrototype%`), reusing the 0030/0034
      iterator approach where possible.
- [ ] WeakMap and WeakSet: keys must be objects (or non-registered symbols,
      symbols-as-weakmap-keys). The copying GC has no weak references yet, so decide:
      implement weakness in the GC (ephemeron tables: an entry is live iff its key is
      reachable otherwise; run to a fixpoint) or make them strong for now and record the
      leak in a decision. Prefer real weakness if it fits the Cheney collector cleanly
      (talk to heap via the coordinator if you need GC changes).
- [ ] Reflect: all 13 functions (no Proxy yet; Reflect works on ordinary and exotic
      objects).
- [ ] Date: the constructor and all prototype methods except the locale ones beyond
      `toLocaleString`-as-`toString`-like formats (no Intl: match qjs output without Intl
      as far as possible), `Date.parse` for the ISO format and the formats qjs's
      toString/toUTCString emit, `Date.UTC`, `Date.now`. Time comes from the host
      (ARCH "Nondeterminism"): a VM hook (`VmOptions`/host callback, or a host-provided
      clock value per run slice) so restores and replays can control it. Local time
      zone: decide (UTC-only unless the host provides an offset function; record it).
      `built-ins/Date` moves from later to active.
- [ ] tests/test262.toml: move to active `built-ins/Symbol` and the Symbol.* features
      that are implemented now (species, toPrimitive, toStringTag, hasInstance,
      isConcatSpreadable, Symbol.prototype.description), `built-ins/Map`, `Set`,
      `WeakMap`, `WeakSet`, the Map/Set iterator prototypes, Reflect, `Reflect.*`, and
      Date. Report active/later numbers before and after the move separately from the
      implementation gains.
- [ ] difftest cases for codemode-typical uses (Map counting, Set dedup, Date
      formatting with a fixed clock), behavior.js comparisons with qjs, e2e cases, and
      the ratchets updated.

## Notes
- Land in parts (for example: the test262.toml promotion + Reflect; Map/Set; Weak*;
  Date).
- heap (0037) is in interp/ and vm/ doing promises and jobs, so keep vm/ changes to the
  clock hook.

## Log

## Follow-ups

## Result
