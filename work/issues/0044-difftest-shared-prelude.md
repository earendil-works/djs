# 0044: djs runs the shared difftest prelude (retire D1160's native console)

status: done
agent: infra
branch: issue/0044-difftest-shared-prelude
depends-on: 0033 (JSON, done), 0041 (await), 0040 (classes, if the prelude uses them)
touches: crates/djs-difftest/, tests/corpus/prelude.js (only if a djs incompatibility must be worked around, with a comment), js/src/sandbox.ts (console formatting, if it has the same issue), ratchets

## Goal
D1160 had djs provide `console`/`text`/`exit`/… natively, because djs could not run
tests/corpus/prelude.js. With JSON (0033 part 1), Promise (0037) and await (0041), it
should be able to now. Running the same prelude on both engines is what makes difftest
compare exactly what the model sees (D1100). The native glue only uses the host-side
`stringify_fast`, so `console/{tojson,objects,numbers-in-json}.js` fail (num, 0033).

## Acceptance criteria
- [x] The djs engine runs the shared prelude and the case like the qjs engine does; the
      native host functions remain only for the primitives the prelude itself needs from
      the host (output, store/load, fake tools), now as async host functions where pi's
      are async (`tools.*`).
- [x] The D1160 known difference (`tools.*` returning directly) is gone.
- [x] Ratchet updated: report how many cases changed (both directions) and why.
- [x] Same for the Sandbox if it formats console output on the host side (check pi's
      prelude behavior, D1100).

## Also (small, from 0045)
- djs-difftest's own codemode setup: `ALL_TOOLS` lacks Array.prototype; `tools` members
  are not enumerable (rest destructuring gives nothing); values print through the fast
  JSON path only (proxies print as `<djs: … Slow>`). All of this should disappear with
  the shared prelude; check it.
- `Object.prototype.__proto__` accessor (Annex B, but real code and `const {__proto__: p}`
  patterns use it; qjs has it).
- `Function.prototype.toString` of a callable proxy shows the target's name like qjs.

## Log
- The djs engine now runs the shared prelude like qjs_driver.js (D1440); host functions
  are only `host.output` and `host.done`. Workarounds: djs Scripts have no completion
  value (the prelude and the case Script store theirs in a global that the engine reads
  and deletes); String.prototype.split does not exist yet (0033 has it in progress), so
  the prelude's errorText splits with a loop (commented; remove once split lands). No
  classes needed: the prelude uses none.
- First run: every djs case failed to set up, then 15 regressed because errorText threw
  (split), then codemode/top-level.js (`this` in the body: the case is now an arrow in a
  Script, as on qjs). A GC-stress crash was the engine holding the global object's
  Value across a run (it moves).
- Tools: kept as the prelude's JS (promise jobs), as on qjs, instead of async host
  functions; see D1440 for why. D1160's difference (tools returning directly) was
  already gone with 0041 and stays gone.
- Ratchet (tests/corpus/expected-failures.txt): 59 -> 49, 10 fixed, 0 new. Fixed:
  codemode/store.js and codemode/tools.js (the native glue's stringify_fast-only store
  and its tools object), console/cyclic-array.js, console/numbers-in-json.js,
  console/objects.js, console/tojson.js, errors/undescribable.js, result/date.js,
  result/tojson-throws.js, result/tojson.js (all values the host-side formatter could
  not show: toJSON, Date, getters, cycles, undescribable throws).
- Stress: all cases identical under --suspend-every 97 and --gc-stress. Each case now
  also evaluates the prelude, so the gate's difftest step costs about twice the CPU
  (wall time 21 -> 22 s in release on 18 cores). Under stress, console/cyclic-array.js
  and builtins/array-methods.js now hit the stress timeout (not compared, as
  dquickts/builtin/test_rope.js already did).
- Sandbox (js/src/sandbox.ts): no host-side formatting to fix; it passes console
  arguments to the embedder's console as values (Opaque values as `[Class]`), and pi
  installs its own in-VM console from its prelude.
- The 0045 leftovers: the tools/ALL_TOOLS differences disappeared with the shared
  prelude (codemode/tools-proxy.js now guards the real tools and maps ALL_TOOLS, prints
  the proxies, rest-destructures the guarded tools). Object.prototype.__proto__ and
  callable-proxy toString (D1441), case builtins/proto-accessor.js. test262 unchanged
  (annexB is skipped; no active test changed).
- djs-difftest's runner test: tools/fail now compared with qjs instead of djs-only
  expectations, plus slow-path JSON values and a stalled case.

## Follow-ups
- Remove `splitLines` from tests/corpus/prelude.js once 0033's String.prototype.split
  is on main (pi's errorText uses `split("\n")`).
- If difftest should model pi's tool timing (results after the job queue drains), both
  engines need it: qjs_driver.js would settle tools from a timer and djs through async
  host functions. Today both resolve them through promise jobs (D1440).
- Under --gc-stress / --suspend-every 97, console/cyclic-array.js and
  builtins/array-methods.js now exceed the stress timeout (reported, not compared).

## Result
djs-difftest's djs engine runs tests/corpus/prelude.js and the case exactly like the qjs
driver (D1440, superseding D1160): only `host.output`/`host.done` are host functions, the
native formatter and its `<djs: ...>` markers are gone. Ratchet 59 -> 49 (10 fixed, none
new), all cases stress-identical. Also Object.prototype.__proto__ (Annex B) and
Function.prototype.toString of callable proxies (D1441), with corpus cases. Known gaps:
the prelude's split workaround until 0033 lands; tools stay JS (promise jobs) on both
engines.
