# atob, btoa and performance.now in codemode (0181)

Question (owner): should djs add `atob`/`btoa` and `performance.now`, which pi's code mode
has? Short answer: pi's scripts on djs already have all three. `atob`/`btoa` are engine
builtins (0084, D1840) that match the spec; `performance.now` comes from the pi adapter
(js/pi/vm.ts `HOST_GLOBALS`, since 0049), with the same clock semantics as pi's QuickJS VM. No
engine change is needed.

## What pi's scripts see today

pi's code mode runs scripts in quickjs-wasi 3.6.2 (pi-private packages/codemode/src/runtime/vm.ts).
`QuickJS.create` takes the default intrinsics (`Intrinsics.ALL`). In deterministic mode it
passes `ALL & ~WEAK_REF`. Both include `PERFORMANCE`, `DOM_EXCEPTION` and `ATOB_BTOA`. pi's
prelude neither adds nor removes any of them. Probed in quickjs-wasi:

- `atob`, `btoa`, `DOMException`: present, length 1, non-enumerable.
- `performance`: a plain object with own `now` and `timeOrigin`.
  - Normal mode: `now()` reads the WASI monotonic clock and `timeOrigin` is epoch ms.
  - Deterministic mode: `now()` is `clock.now - startedAt`, which starts at 0 and moves
    only with delivered results, as Date does. pi tests this
    (test/deterministic.test.ts). `timeOrigin` reads 0 there, because it is read while
    the context is created, when the realtime clock is seed-derived.
- `performance.now()` in pi's own host code (execution.ts) times tool calls on the host. It is
  not what scripts see.

djs's pi adapter supplies the same set. `atob`/`btoa`/`DOMException` are native.
`performance` is `{ now }`, defined before the prelude so the lockdown freezes it. `now()`
is `Date.now() - origin` on the VM clock, with the origin set per run: `startedAt` in
deterministic mode (giving 0, then the delivered result times, as on quickjs) and the run's
start otherwise. The origin lives in a closure, so it is heap data. Snapshots, dumps and
resumes keep it, and the prelude image doesn't depend on it. js/test/pi-vm.test.ts covers
it. The only difference from quickjs-wasi is that djs has no `performance.timeOrigin`.

## Usage in harvested scripts

Counts from the 0149 extract (2,782 scripts, 2,687 distinct; 0130's 2,627 give the same
picture), regex over the script text:

| pattern | scripts |
|---|---:|
| `atob(` / `btoa(` / `DOMException` | 0 / 0 / 0 |
| `performance.now` | 3 (all succeeded on quickjs; elapsed-time reporting) |
| `performance.timeOrigin` | 0 |
| `performance` as any word | 21 (the others are prose in strings) |
| `Date.now` / `new Date` | 88 / 79 |
| `Buffer` / `TextEncoder` | 4 / 1 (not in pi either) |

The p2 extract (25 scripts) has no use of any of these.

## atob/btoa conformance

work/scratch/0181/b64.js compares 46 cases across node, quickjs-wasi (pi), dquickts qjs and
djs: forgiving-base64 whitespace (only `\t\n\f\r` and space; `\v`, NBSP and U+3000 are
errors), optional and misplaced padding, `len % 4 == 1`, ignored trailing bits, the URL
alphabet, non-ASCII input, Latin-1 output, ToString of null/undefined/numbers/objects,
Symbol, a lone surrogate, the 256-byte round trip, and the descriptors. djs matches dquickts
qjs on every line. It matches node on every data case, including the
`DOMException`/`InvalidCharacterError`/code 5 errors. The differences:

- `atob()` / `btoa()` without an argument: djs, qjs and node throw a TypeError (WebIDL).
  pi's quickjs-wasi treats the missing argument as `undefined`: `atob()` throws
  InvalidCharacterError and `btoa()` returns `"dW5kZWZpbmVk"`. No harvested script does this.
- Enumerability: the globals are non-enumerable in djs, qjs and quickjs-wasi, and enumerable
  in node and WebIDL. We keep qjs's behavior; changing it buys nothing.
- `new atob()`: a TypeError (WebIDL operations aren't constructors); node constructs.

The cases are now in the difftest corpus as tests/corpus/builtins/base64-edges.js
(expected output from qjs; djs passes plain, suspended and under GC stress). The existing
tests are the e2e web_globals.js case, the imported dquickts test_base64.js and the unit
tests in builtins/dom.rs.

## performance.now as an engine builtin?

No. The analysis:

- **Clock.** The engine has one clock, the host-set `Clock` realm slot that Date reads
  (D1383), and it never reads a system clock. An engine `performance.now()` could only
  return that clock minus a stored origin, which is what the adapter's three lines already
  do.
- **Determinism.** It is the same either way. In deterministic mode the clock moves only
  with delivered async results (D1680), and fuel slices don't move it, so `now()` is a
  function of `startedAt` and the result times on any fuel slicing.
- **Fuel.** One builtin call, as for Date.now.
- **Snapshots and dumps.** An engine origin would need a new realm slot (an image change) and a
  Vm/wasm setter. The adapter's closure is already ordinary heap data.
- **Resolution.** The clock has ms resolution, and outside deterministic mode the adapter
  sets it once per slice (10,000 fuel). A tight loop therefore sees `now()` advance in
  steps, not continuously. That is fine for the elapsed-time reporting the harvested
  scripts do. A finer clock would need the host to pass a fractional time (Date must stay
  integral), and nothing asks for it.
- **Size and slim build.** A builtin would cost an object, a function and a prelude entry,
  roughly 0.5–1 KB of module and image. That isn't much, but every djs host would pay it
  for a global that only pi's quickjs-wasi parity needs. The slim build's keep list
  (docs/slim-codemode.md) doesn't change either way, because the adapter defines
  `performance` on top of any build.
- **Members.** Keep only `now`, which is what scripts use. Leave out `mark`/`measure`/
  `getEntries`. `timeOrigin` has no users. If parity is wanted, it could be a getter on
  the adapter's object that returns 0 in deterministic mode (as pi's quickjs does) and the
  run's start otherwise.

## Recommendation and plan

1. No engine work: atob/btoa already conform and performance.now is already provided.
2. Done here: the base64 edge-case difftest case.
3. Optional, adapter only (js/pi/vm.ts, one line plus a pi-vm test): `performance.timeOrigin`
   for full quickjs-wasi parity. `HOST_GLOBALS_HASH` changes the deterministic fingerprint
   automatically. It is low value because no script uses it.
4. Not recommended: making atob/btoa enumerable, and copying quickjs-wasi's lenient missing
   argument. Both would move djs away from qjs, the difftest oracle, for no user benefit.
