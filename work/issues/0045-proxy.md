# 0045: Proxy

status: done
agent: infra
branch: issue/0045-proxy
depends-on: 0038 (Reflect)
touches: crates/djs-core/src/interp/ (property-operation dispatch for proxy objects: get/set/has/delete/defineOwnProperty/getOwnPropertyDescriptor/ownKeys/getPrototypeOf/setPrototypeOf/isExtensible/preventExtensions/call/construct), crates/djs-core/src/heap/ (a Proxy object class, additive), builtins/proxy.ts (new) + the internal-method helpers in builtins/object.ts etc., tests/test262.toml (promote Proxy/Reflect-with-proxies), ratchets

## Goal
Proxy is the last big piece of the object model. pi's codemode prelude wraps `tools` in
a Proxy (prelude-source.ts), and test262 has a large Proxy suite (later group). All 13
traps with the full invariant checks, revocable proxies, and proxies as prototypes and
in every builtin that reaches internal methods.

## Acceptance criteria
- [x] A Proxy object class (target, handler, revoked state) with callable/constructible
      forms. The interpreter's property paths (fast paths stay fast: the proxy check must
      be a cheap class test on the slow path) dispatch to trap calls. Traps run user code,
      so they go through the privileged JS slow paths (D1085): `%ProxyGet`, `%ProxySet`, …
      in the builtin language, with the invariant checks per spec.
- [x] `Proxy`, `Proxy.revocable`. Every builtin that uses internal methods (Object.keys,
      JSON.stringify, Array.isArray through proxies, for-in, spread, Reflect.*, …) works
      on proxies, which mostly means using the `%Get`/`%Set`/`%OwnKeys` intrinsics that
      already exist and are proxy-aware. Audit the leaf fast paths that assume ordinary
      objects (the D1311/0012 "Slow" signals: stringify_fast must bail on proxies).
- [x] Promote built-ins/Proxy and the `Proxy` / `proxy-missing-checks` features to
      active; report the numbers before and after the move.
- [x] behavior.js comparisons with logging handlers (the order of trap calls matters and
      test262 checks it), difftest cases (a pi-prelude-like tools proxy), e2e cases in
      the three modes, and the ratchets.
- [x] Performance: no measurable regression on the engine track (`scripts/bench.sh`).

## Notes
- num (0033 part 2, RegExp) and heap (0043, wasm size) are active. heap's work is in the
  image and compiler options, not the property paths. Coordinate interp changes via the
  coordinator if they get large.

## Log
- test262 before the move (main 23be57f): active 18,484 / 24,120 (76.6%), later 252 /
  13,449. The move alone (same engine): active 18,484 / 24,497 (75.4%), later 252 /
  13,072: +377 active tests (built-ins/Proxy 258, Proxy feature 375 in total,
  proxy-missing-checks 3), none passing.
- ObjectClass::Proxy = 27 (told the coordinator).
- Design (D1450): an empty, non-extensible heap cell; `lookup_property` /
  `has_property_or_proxy` report a proxy on the chain; the interpreter's property ops and
  the Get/Set/Has(WithReceiver) intrinsics call `%ProxyGet`/`%ProxySetOrThrow`/... in
  builtins/proxy.ts; the other internal-method intrinsics redirect to same-signature
  builtins when their first argument is a proxy (`REDIRECTS`, interp/run/proxy.rs).
- First test262 run after the implementation: built-ins/Proxy 249/258, the rest need
  classes, regexps or eval. Fixed three follow-on failures: `new` with a proxy/getter
  new.target (D1451, replaces "not supported yet: getter for 'prototype'"),
  Object.prototype.toString's IsArray before the Get, primitive [[Set]] through a proxy
  prototype (D1453).
- behavior.js: our proxies behind native qjs shells, against native proxies, with
  logging handlers for all 13 traps and one invariant-breaking trap at a time, over 39
  operations x 6 targets: trap order and messages match (messages taken from qjs).
  Shell artifacts left out: proxies of proxies (the shells' own checks run inner traps
  again; covered in difftest), a revoked shell's IsArray/IsCallable, and the
  target-vs-proxy wording of a failed strict [[Set]].
- for-in: first eager (all keys up front), which dquickts' test_for_in_proxy rejects
  (a key deleted during the loop must be skipped); now lazy with qjs's order (D1452).
- Spec over qjs: Object.isFrozen/isSealed call isExtensible before ownKeys (qjs after).
- Benchmarks (engine track, interleaved runs of base 23be57f and this branch, the
  machine was busy): within noise. Best-of `djs run` wall times base/head: calls
  164.6/167.5 ms (an earlier round 163.6/161.3), props 370.3/369.5, loop 496.8/502.5,
  higher-order 673.5/664.2, arrays 1509/1499. bench.sh rounds likewise (calls best
  102.8-109 vs 104.7-112 under load, props 250-273 both).
- test262 after: active 18,853 / 24,497 (76.9%), from 18,484 (+369: the Proxy suite and
  proxy tests elsewhere, e.g. Reflect, Object.keys/freeze/seal with proxies, Array
  methods over proxied arrays). Features: Proxy 352/375 active (93.8%),
  proxy-missing-checks 3/3, Reflect 276/286, Reflect.construct 404/424. The remaining
  proxy failures all need classes, regexps, async functions or eval.

## Follow-ups
- Private names (0040): they bypass proxies and act on the proxy object itself, but the
  heap's proxy cell is not extensible and refuses stores (D1450's safety net). Private
  field/method definition on a proxy must skip the extensibility check (PrivateFieldAdd
  ignores it anyway) and own_keys must keep skipping private keys.
- djs-difftest's own codemode glue (0044 replaces it with the shared prelude): ALL_TOOLS
  has no Array prototype (`ALL_TOOLS.map` is "not a function"), tool results are not
  promises, `tools` has Object.prototype and its members are not enumerable, rest
  destructuring of `tools` is empty, and values (console, text, return) are formatted
  by stringify_fast only, so a proxy prints `<djs: JSON.stringify needs the slow path>`.
  tests/corpus/codemode/tools-proxy.js builds its own frozen null-prototype tools object
  for that reason.
- Function.prototype.toString of a callable proxy gives `function () { [native code] }`;
  qjs prints the target's name.
- Object.prototype.__proto__ (Annex B) does not exist (noticed while testing).

## Result
Landed: Proxy and Proxy.revocable with all 13 traps and their invariant checks
(builtins/proxy.ts, D1450), callable and constructible proxies (`typeof`, calls, `new`,
Reflect.apply/construct, bind), proxies as prototypes, handlers and targets, revocation.
Every builtin over the internal-method intrinsics works on proxies (Object.keys/values/
entries/assign/freeze/seal/defineProperty/getOwnPropertyDescriptors, Reflect.*,
JSON.stringify, Array.isArray, Array methods, spread, rest, for-in, `in`, `delete`,
`instanceof`, Object.prototype.toString). stringify_fast bails on proxies. Also: `new`
with a `prototype` getter or proxy new.target (D1451), primitive [[Set]] through
prototype setters/proxies (D1453), lazy for-in through proxies (D1452).

Tests: behavior.js (proxies behind native shells against qjs, ~5,000 comparisons of trap
logs and invariant errors), tests/corpus/builtins/proxy.js and codemode/tools-proxy.js
(a pi-like `guard` over tools and a namespace), tests/data/e2e/proxies.js (three modes),
dquickts' test_for_in_proxy passes. test262 Proxy promoted to active: 18,484 -> 18,853
active passes; Proxy feature 352/375. Ratchets updated. No engine benchmark regression.

Known gaps: see Follow-ups (private names on proxies with 0040, the difftest glue,
Function.prototype.toString naming); for a proxy further up a for-in chain qjs reads its
keys twice and we do not.
