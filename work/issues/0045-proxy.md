# 0045: Proxy

status: open
agent: -
branch: issue/0045-proxy
depends-on: 0038 (Reflect)
touches: crates/djs-core/src/interp/ (property-operation dispatch for proxy objects: get/set/has/delete/defineOwnProperty/getOwnPropertyDescriptor/ownKeys/getPrototypeOf/setPrototypeOf/isExtensible/preventExtensions/call/construct), crates/djs-core/src/heap/ (a Proxy object class, additive), builtins/proxy.ts (new) + the internal-method helpers in builtins/object.ts etc., tests/test262.toml (promote Proxy/Reflect-with-proxies), ratchets

## Goal
Proxy is the last big piece of the object model. pi's codemode prelude wraps `tools` in
a Proxy (prelude-source.ts), and test262 has a large Proxy suite (later group). All 13
traps with the full invariant checks, revocable proxies, and proxies as prototypes and
in every builtin that reaches internal methods.

## Acceptance criteria
- [ ] A Proxy object class (target, handler, revoked state) with callable/constructible
      forms. The interpreter's property paths (fast paths stay fast: the proxy check must
      be a cheap class test on the slow path) dispatch to trap calls. Traps run user code,
      so they go through the privileged JS slow paths (D1085): `%ProxyGet`, `%ProxySet`, …
      in the builtin language, with the invariant checks per spec.
- [ ] `Proxy`, `Proxy.revocable`. Every builtin that uses internal methods (Object.keys,
      JSON.stringify, Array.isArray through proxies, for-in, spread, Reflect.*, …) works
      on proxies, which mostly means using the `%Get`/`%Set`/`%OwnKeys` intrinsics that
      already exist and are proxy-aware. Audit the leaf fast paths that assume ordinary
      objects (the D1311/0012 "Slow" signals: stringify_fast must bail on proxies).
- [ ] Promote built-ins/Proxy and the `Proxy` / `proxy-missing-checks` features to
      active; report the numbers before and after the move.
- [ ] behavior.js comparisons with logging handlers (the order of trap calls matters and
      test262 checks it), difftest cases (a pi-prelude-like tools proxy), e2e cases in
      the three modes, and the ratchets.
- [ ] Performance: no measurable regression on the engine track (`scripts/bench.sh`).

## Notes
- num (0033 part 2, RegExp) and heap (0043, wasm size) are active. heap's work is in the
  image and compiler options, not the property paths. Coordinate interp changes via the
  coordinator if they get large.

## Log

## Follow-ups

## Result
