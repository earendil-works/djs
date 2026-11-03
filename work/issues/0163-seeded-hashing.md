# 0163: Seeded hashing for VM atoms and Map/Set

status: done
agent: strings
branch: issue/0163-seeded-hashing
depends-on: 0150
guide: docs/guide/determinism.md, strings or collections pages as affected
touches: crates/djs-core/src/heap/ (string hash, atom index of the VM's own atoms, Map/Set tables), crates/djs-core/src/vm/ (seed from the VM's deterministic seed / options), docs/decisions.md

## Goal
docs/lessons-from-pico.md lesson 5: djs's string hash is fixed and unseeded, so crafted tool
data could make atom-index or Map/Set lookups quadratic. Seed the hash per VM:
- the base image's prefix (atoms, base index, D2431) stays unseeded and build-fixed;
- the VM's own atom index and Map/Set tables use a per-VM seed, derived from the VM's
  deterministic seed when deterministic mode is on (determinism holds: same seed, same
  layout), random otherwise; the seed lives in the image;
- iteration order of Map/Set is insertion order and doesn't change;
- a test that crafted colliding keys for the unseeded hash stay linear.
No workload slower than +1%.

## Log
- 2026-11-09 (strings): design (D2433): SipHash-1-3 over code units for everything; the
  VM's key in `Roots::HASH_KEY`, derived one-way from the `Math.random` seed (Vm::new,
  from_base, set_random_seed); the base index under a fixed key (0). Strings cache their
  level's hash (prefix: base, above: keyed). Rekey = clear cached hashes above the
  prefix, rebuild the VM's atom index and every Map/Set index (order kept).
- First version (+4.5 KB gzip, props +5.9%): trimmed code to +453 B gzip on today's main (one copy of
  the rounds, one probe loop with a needle enum, no dyn iterators, the old fixed hash
  dropped). props' cost was probe-placement luck plus per-step cost: identity match for
  an atom's own string, cached-hash reads per candidate, own index at 1/4 load below
  1,024 buckets. JSON natives looked up "toJSON" through a fresh string per value:
  `n_str` returns the atom's string for toJSON/rawJSON (interp/run/native.rs, outside
  touches, two match arms). `collection_set` caches string keys' hashes.
- pi: per-session rekey added ~1 KB per container; outside deterministic mode pi now
  keys once before the prelude and reseeds Math.random alone per session
  (`Vm::set_math_random_seed`, wasm `vm_set_math_random_seed`, js/pi engine.ts/vm.ts,
  outside touches). Deterministic sessions still rekey (layout = f(seed)).
- Tests: heap/hash_tests.rs (2,048 keys colliding under the old hash spread out in the
  VM's atom index and in a Map; rekeying keeps atoms, Maps and order),
  vm::tests::the_hash_key_follows_the_seed (same seed same image, other seed same
  results, reseeding mid-session). Checks: djs-core lib, djs-prelude, djs-cli, e2e,
  difftest corpus + codemode stress/determinism, js tests, native-vs-wasm, test262
  Map/Set/WeakMap/JSON/Symbol/computed-property-names, registry check (no update).
- Guide: determinism.md (sources table, deterministic mode), heap.md (atoms, Map/Set).
- Rebased onto 0164 (D2265): `Roots::HASH_KEY` moved to 104 (Roots 112 B),
  `vm::COMPAT_VERSION` 1 → 2 (stored hashes change meaning; the SipHash constants are not
  top-level consts, so RECORDS_HASH does not see them; `BASE_HASH_KEY` is one and is).

## Follow-ups
- Map with number keys pays a SipHash per operation (+4.3% on a Map-int micro, not in the
  perf workloads): a cheaper keyed mixer for single words could do, if judged strong
  enough.
- Deterministic pi sessions rekey the prelude's atoms and tables per session (+1 KB per
  container). Keying the prelude image per seed would need a cache per seed.

## Result
- Keyed SipHash-1-3 for the VM's own atoms, strings and Map/Set keys, key in the image,
  derived from the seed (D2433); the base prefix keeps a build-fixed key. Insertion order
  unchanged. `Vm::set_math_random_seed` for hosts that key once (pi).
- Crafted colliding keys stay linear (heap/hash_tests.rs).
- Perf: native workloads ≤ +0.31%, wasm within noise; Map-int micro +4.3%.
- Size: +453 B gzip (444,523 → 444,976), slim +434 B; ceilings unchanged.
- pi deterministic containers +1 KB; non-deterministic unchanged path (no per-session rekey).
