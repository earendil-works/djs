# 0036: P1-6 Destructuring, tagged templates, remaining spread/rest

status: open
agent: -
branch: issue/0036-p1-destructuring-templates
depends-on: 0029
touches: crates/djs-core/src/compiler/, crates/djs-core/src/bytecode/ (additive), builtins/ (only if a helper is needed, e.g. %CopyDataProperties for object rest), docs/compiler.md

## Goal
docs/compiler.md section 10, row P1-6: binding and assignment patterns everywhere (`let
{a, b: [c, d = 1], ...rest} = x`, parameters, `catch (e)` patterns, for-in/of heads, and
assignment `[a, b] = [b, a]` with the 3.3 memoized pattern scan replacing the minimal
check), defaults, holes and elisions, object rest (copy with excluded keys), array rest,
and iterator closing on abrupt completion during array destructuring (the same unwind
mechanism as for-of). Tagged templates (cooked/raw, invalid escapes → undefined cooked,
the per-site template object cache, frozen arrays with `raw`). Codemode scripts use
destructuring constantly, and test262 has ~1,500 tests behind it (0007 clusters).

## Acceptance criteria
- [ ] All pattern positions above, with spec evaluation order (property keys, defaults,
      and iterator steps interleaved correctly), and early errors (invalid targets,
      rest not last, rest with an initializer, duplicate `__proto__` only outside
      patterns, which is already done).
- [ ] Interpreter needs (for example a copy-with-excluded-keys op or intrinsic, or the
      template object cache in the realm) are sent to the coordinator early as one
      list. heap is on 0035 (perf) in interp/. If an item is tiny and additive, you may
      do it yourself and log it.
- [ ] e2e tests checked against qjs, a disassembly snapshot, the test262 compile sweep
      before/after, and the ratchets updated.

## Log

## Follow-ups

## Result
