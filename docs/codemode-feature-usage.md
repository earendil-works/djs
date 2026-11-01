# Which JavaScript features does codemode use?

Issue 0130, measured at main 74353e58. The owner asked which "extended"
JavaScript features real codemode scripts need, as evidence for what djs could drop or
demote. This is research only: no engine changes. Sizes are bytes of the shipped wasm
module; "gz" is gzip -9.

## In short

- **Data.** Every codemode execution in the owner's pi sessions: 2,627 executions,
  2,535 distinct scripts, 348 sessions, 2026-09-25 to 2026-11-01. Each script was
  compiled with djs's own compiler, and features were read off its bytecode. A token
  scan covers what the bytecode does not show and also serves as a cross-check.
- **What scripts use** is a narrow, modern subset of JavaScript:
  - `await` on tool calls (98% of scripts) and `Promise.all`/`allSettled` (28%);
  - arrows and template literals;
  - for-of and destructuring;
  - `?.` and `??`, and try/catch;
  - JSON;
  - regex literals with `g`/`i`;
  - string methods (slice, split, replace, trim);
  - array methods (map, join, filter, push, sort, find, reduce);
  - `Object.entries`/`keys`, `Date.now`/`new Date`/`toISOString`, `Math`, and `Set`.
- **Never used, in 2,535 scripts:**
  - **Object model:** classes (any form), getters/setters, `super`, `new.target`,
    `arguments`, `instanceof`, `in` outside for-in, Proxy, Reflect, Symbol, and every
    `Object.*` reflection call (`defineProperty`, `freeze`, `create`, `getPrototypeOf`,
    and so on).
  - **Iteration:** generators and async generators, for-await, `yield*`, Iterator
    helpers.
  - **Builtin families:** WeakMap/WeakSet/WeakRef/FinalizationRegistry, BigInt, typed
    arrays/ArrayBuffer/DataView, `normalize`.
  - **Dynamic code and newer syntax:** tagged templates (including `String.raw`),
    `eval`/`Function`, labels, `using`.
  - **Sloppy mode:** no `var`, no top-level `this`, no implicit globals, no `with`, no
    octal literals.
  - **Regular expressions:** named groups, `\p{…}`, backreferences, lookahead, and the
    `u`/`y`/`d`/`v` flags. Lookbehind appears once.
  - **Globals:** no `encodeURIComponent`, `atob`/`btoa` or `escape`.
- **Rare** (1–15 scripts): for-in, object rest, `delete`, `switch`, `do`/`while`,
  logical assignment, `Map`, `new RegExp`, lazy quantifiers, the `m`/`s` flags,
  lookbehind, `localeCompare`, `flatMap`, `at`, `padStart`/`padEnd`, `parseInt`.
- **pi's own prelude needs much of what scripts never use.** It uses Proxy and Reflect
  (the `tools` object is a Proxy, so 92% of scripts go through a Proxy trap),
  getters/setters, `Object.defineProperty`/`freeze`/`getOwnPropertyDescriptors`,
  Symbol.iterator/matchAll, and `function*`/`async function*` syntax. It also needs
  `Int8Array` to exist, because it freezes `%TypedArray%`. Those features stay, or pi's
  prelude changes.
- **Biggest measured win:** the two Unicode features already behind cargo features.
  Turning off `\p{…}` (`regexp-unicode-props`) saves 21.0 KB gz. Turning off full
  normalization (`unicode-normalization`) saves 19.8 KB gz. Together they save 40.7 KB
  gz (8.8%) and bring the module from 460.9 to 420.1 KB gz. That is 1.47x
  quickjs-wasi, under the 1.5x budget (D020). No script used either feature;
  normalization only reaches scripts through `localeCompare` on non-ASCII text.
- **Next tier (estimated):** typed arrays/ArrayBuffer/DataView (~13 KB gz), BigInt
  (~10 KB gz plus a branch in every numeric operator), Iterator helpers (~5 KB gz),
  decorators (~5 KB gz), `using`/DisposableStack (~3 KB gz), weak references (~2 KB gz,
  but more GC complexity), eval/Function (~1 KB gz). Classes (~7.5 KB gz) and
  generators are never used, but they are core language features models do write in
  longer programs. They are not cut candidates.
- **Risks:**
  - The sample is one owner's sessions over five weeks.
  - pi's prelude depends on several of these features.
  - test262 would lose thousands of tests per cut family.
  - A model that does reach for a dropped feature gets a hard error instead of a
    slower path.

## 1. Data and method

The session reader is the harvester's (0118, 0121): `scripts/harvest/extract.py` pairs
each codemode tool call with its result entry and now also records the result's time.
`scripts/harvest/features.py` compiles each distinct script the way pi runs it: the body
of an async function with `tools` and `console` as parameters (`djs disasm --body`). It
reads features from the bytecode:

- **opcodes:** `define_class`, `generator_start`, `special_object arguments`,
  `get_template_object`, `delete`, `in`, `instanceof`, `for_in_start`, the spread and
  rest ops, `define_getter`/`define_setter`, private-field ops, super ops, `pow`, BigInt
  constants;
- **intrinsic calls:** `%GetIterator`, `%GetAsyncIterator`, `%RegExpCreate` (whose
  constant pattern and flags are then parsed for the regex syntax used),
  `%CopyDataPropertiesExcluding`, the dispose intrinsics;
- **global reads:** each builtin global, and `Global.member` statics when a `get_field` or
  `get_method` follows directly. An assignment to an undeclared global would show up as
  `put_global`.
- **member calls and reads:** the name of every `.name(` call and `.name` read.
  Receivers are not known statically: `.description` is a tool's field, not
  `Symbol.prototype.description`.

The bytecode cannot show labels, `new.target` written out, sloppy-only tokens (octal
literals, `with`) or which keywords a script uses, so a token pass adds those
(scripts/harvest/jstok.py). The summary counts executions, distinct scripts, sessions,
and the first and last date per feature. It also marks the features pi's prelude uses
itself, by running the same analysis on `PRELUDE_SOURCE` from pi-private.

Checks:

- **qjs agreement.** 12 scripts (0.5%) don't compile in djs. All 12 also fail in qjs:
  most are Python written into the JavaScript tool, the rest are broken scripts. No script
  fails in djs alone, so nothing that models write is already missing from djs.
- **Token cross-check.** A separate token scan for every "never" feature found the same
  zeros as the bytecode. Its hits were two variables named like keywords, and `in`
  inside for-in heads or in the 12 non-JavaScript scripts.
- **Why not regexes.** A plain regex scan over the sources overcounts badly, because
  matches inside strings and embedded programs count too. It reported `BigInt` in 21
  scripts, `class` in 2, `fetch` in 21, `require` in 14 and `setTimeout` in 16. The
  bytecode shows none of these as code.
- **qjs-only globals.** One script called qjs's `print`.

Only the owner's machine was scanned. The sessions are mostly pi's own development work
plus Blender, web search, artifacts, git triage and model-classification tasks. The task
mix shapes what shows up: no binary data, no heavy text processing in other scripts.

## 2. Usage

Counts are distinct scripts and sessions. "prelude" marks features pi's prelude uses itself.

### Syntax

| feature | scripts | sessions | first | last | prelude |
| --- | ---: | ---: | --- | --- | --- |
| `await` in the body | 2,481 | – | | | yes |
| template literal | 998 | 196 | 2026-09-25 | 2026-11-01 | |
| arrow function | 895 | 233 | 2026-09-25 | 2026-11-01 | yes |
| iteration protocol (for-of, array destructuring) | 765 | 196 | 2026-09-25 | 2026-11-01 | yes |
| nested async function | 290 | 59 | 2026-09-25 | 2026-10-31 | yes |
| `??` | 271 | 92 | 2026-09-25 | 2026-11-01 | |
| `?:` | 232 | 113 | 2026-09-25 | 2026-11-01 | yes |
| `?.` | 223 | 75 | 2026-09-28 | 2026-11-01 | |
| `let` | 200 | 63 | 2026-09-25 | 2026-11-01 | yes |
| regex literal | 166 | 57 | 2026-09-25 | 2026-11-01 | yes |
| destructuring declaration | 147 | 72 | 2026-09-25 | 2026-10-31 | yes |
| `typeof` | 132 | 49 | 2026-09-29 | 2026-11-01 | yes |
| `new` | 105 | 26 | 2026-09-28 | 2026-10-31 | yes |
| object spread | 91 | 30 | 2026-09-25 | 2026-10-31 | |
| try/catch | 71 | 32 | 2026-09-25 | 2026-11-01 | yes |
| `throw` | 74 | 15 | 2026-09-28 | 2026-10-31 | yes |
| function declaration or expression | 52 | 9 | 2026-09-25 | 2026-10-31 | yes |
| array/call spread | 33 | 23 | 2026-09-25 | 2026-10-30 | yes |
| `while` | 16 | 12 | 2026-09-25 | 2026-11-01 | yes |
| logical assignment (`??=` …) | 15 | 7 | 2026-09-25 | 2026-10-25 | |
| for-in | 4 | 3 | 2026-10-01 | 2026-10-30 | |
| `do`/`while` | 3 | 2 | 2026-09-30 | 2026-09-30 | |
| object rest | 2 | 2 | 2026-10-04 | 2026-10-09 | |
| `switch` | 1 | 1 | 2026-09-29 | 2026-09-29 | |
| `delete` | 1 | 1 | 2026-10-25 | 2026-10-25 | |
| class (declaration, expression, fields, private, static, extends) | 0 | 0 | | | |
| getter/setter (literal or class) | 0 | 0 | | | yes |
| `super`, `new.target` | 0 | 0 | | | |
| generator, async generator, `yield*` | 0 | 0 | | | yes (syntax only) |
| for-await | 0 | 0 | | | |
| `arguments` | 0 | 0 | | | |
| `in` (outside for-in), `instanceof` | 0 | 0 | | | yes |
| tagged template (incl. `String.raw`) | 0 | 0 | | | |
| labels | 0 | 0 | | | |
| `using` / `await using` | 0 | 0 | | | |
| `**` | 0 | 0 | | | |
| BigInt literal | 0 | 0 | | | |
| `var`, `debugger` | 0 | 0 | | | |
| sloppy mode: top-level `this`, implicit global, `with`, octal | 0 | 0 | | | |

### Regular expressions

Constant patterns (literals and `new RegExp` with a constant), parsed for their syntax:

| feature | scripts | sessions | first | last | prelude |
| --- | ---: | ---: | --- | --- | --- |
| flag `g` | 98 | 37 | 2026-09-28 | 2026-11-01 | yes |
| flag `i` | 33 | 19 | 2026-09-28 | 2026-10-30 | |
| lazy quantifier | 7 | 5 | 2026-10-08 | 2026-10-30 | |
| flag `m` | 6 | 3 | 2026-09-25 | 2026-10-25 | |
| flag `s` | 1 | 1 | 2026-10-26 | 2026-10-26 | |
| lookbehind | 1 | 1 | 2026-10-11 | 2026-10-11 | |
| lookahead | 0 | 0 | | | yes |
| named groups, `\k<…>`, backreferences | 0 | 0 | | | |
| `\p{…}` / `\P{…}` | 0 | 0 | | | |
| flags `u`, `y`, `d`, `v`; `v` set operations; modifiers | 0 | 0 | | | |

Through String methods, `replace`/`split`/`match`/`matchAll`/`replaceAll` with a regex
(136 scripts) go through the `Symbol.replace`/`split`/`match` protocol.

### Builtins

| global or static | scripts | sessions | first | last | prelude |
| --- | ---: | ---: | --- | --- | --- |
| `Promise` | 711 | 188 | 2026-09-25 | 2026-10-31 | yes |
| `Promise.allSettled` | 451 | 103 | 2026-09-28 | 2026-10-31 | |
| `JSON.stringify` | 263 | 63 | 2026-09-28 | 2026-11-01 | yes |
| `Promise.all` | 261 | 104 | 2026-09-25 | 2026-10-31 | |
| `JSON.parse` | 132 | 42 | 2026-09-28 | 2026-10-31 | yes |
| `String` (call) | 116 | 65 | 2026-09-25 | 2026-11-01 | yes |
| `Error` (70 of them `new Error`) | 74 | 15 | 2026-09-28 | 2026-10-31 | yes |
| `Object.entries` | 64 | 31 | 2026-10-01 | 2026-10-31 | yes |
| `Date` (22 `new Date`, 25 `Date.now`, 10 `Date.parse`) | 43 | 15 | 2026-09-25 | 2026-10-19 | |
| `Object.keys` | 31 | 15 | 2026-09-29 | 2026-10-30 | yes |
| `Math` (max, floor, round, min, abs, ceil, random) | 30 | 18 | 2026-09-25 | 2026-11-01 | |
| `Boolean` | 16 | 12 | 2026-09-25 | 2026-10-25 | |
| `Set` | 14 | 12 | 2026-09-28 | 2026-10-30 | yes |
| `Number` | 13 | 6 | 2026-10-04 | 2026-10-30 | |
| `Object.fromEntries` | 10 | 3 | 2026-10-01 | 2026-10-25 | |
| `Array.isArray`, `Array.from` | 9 | 6 | 2026-09-25 | 2026-10-27 | yes |
| `Object.values` | 5 | 3 | 2026-10-16 | 2026-10-30 | |
| `parseInt` | 4 | 3 | 2026-09-29 | 2026-10-17 | |
| `Map` | 3 | 2 | 2026-09-28 | 2026-09-30 | yes |
| `new RegExp` | 3 | 2 | 2026-10-13 | 2026-10-25 | |
| `isFinite` | 1 | 1 | 2026-10-10 | 2026-10-10 | |
| `Proxy`, `Reflect`, `Symbol`, `globalThis` | 0 | 0 | | | yes |
| `Object.defineProperty`/`freeze`/`create`/`getPrototypeOf`/… | 0 | 0 | | | yes |
| `Promise.race`/`any`/`withResolvers`, `new Promise` | 0 | 0 | | | |
| `Iterator` (helpers, `Iterator.from`) | 0 | 0 | | | guarded |
| typed arrays, `ArrayBuffer`, `DataView` | 0 | 0 | | | `Int8Array` |
| `BigInt` | 0 | 0 | | | |
| `WeakMap`, `WeakSet`, `WeakRef`, `FinalizationRegistry` | 0 | 0 | | | |
| `DisposableStack`, `AsyncDisposableStack`, `SuppressedError` | 0 | 0 | | | |
| `eval`, `Function` | 0 | 0 | | | |
| other error constructors (`TypeError`, …) | 0 | 0 | | | yes |
| `encodeURI*`/`decodeURI*`, `escape`/`unescape`, `atob`/`btoa` | 0 | 0 | | | |
| `queueMicrotask`, `DOMException` | 0 | 0 | | | |

Prototype methods, by name: the receiver is unknown statically, so the name may also be
a tool result's field.

| method | scripts | sessions |
| --- | ---: | ---: |
| `.map` | 694 | 206 |
| `.join` | 443 | 143 |
| `.slice` | 432 | 116 |
| `.push` | 215 | 83 |
| `.length` (read) | 181 | 75 |
| `.replace` | 170 | 48 |
| `.split` | 127 | 59 |
| `.forEach` | 110 | 38 |
| `.filter` | 92 | 45 |
| `.includes` | 80 | 36 |
| `.trim` | 79 | 38 |
| `.indexOf` | 63 | 21 |
| `.message` (read) | 56 | 26 |
| `.test` | 55 | 26 |
| `.toFixed` | 51 | 23 |
| `.catch`, `.then` | 47, 38 | 27, 14 |
| `.startsWith`, `.endsWith` | 36, 14 | 22, 11 |
| `.sort` | 36 | 19 |
| `.find` | 27 | 17 |
| `.toISOString` | 22 | 6 |
| `.match`, `.matchAll`, `.exec` | 16, 8, 8 | 12, 4, 3 |
| `.reduce` | 14 | 11 |
| `.replaceAll`, `.trimEnd` | 12, 12 | 7, 6 |
| `.some`, `.at`, `.entries`, `.toLowerCase`, `.localeCompare` | 7, 6, 6, 6, 6 | 5, 4, 4, 5, 3 |
| `.has`/`.add`/`.get`/`.set`/`.size` (Set, Map) | 4–7 | 1–5 |
| `.findIndex`, `.flatMap`, `.repeat`, `.padEnd`, `.pop` | 3–4 | 2–3 |
| `.findLastIndex`, `.getUTCDay`, `.flat`, `.splice`, `.concat`, `.padStart`, `.lastIndexOf` | 1–2 | 1 |
| `.stack` (read) | 1 | 1 |
| `.next` (a hand-written iterator) | 0 | 0 |
| `.normalize`, `.toLocaleString`, `.toUpperCase`, `.substring`, `.charCodeAt`, `.codePointAt` | 0 | 0 |
| `.every`, `.hasOwnProperty`, `.call`, `.apply`, `.bind`, `.finally` | 0 | 0 |
| `.getTime`, `.getFullYear` and the other local-time Date getters | 0 | 0 |

## 3. What pi's prelude needs

pi's `PRELUDE_SOURCE` (pi-private `packages/codemode/src/runtime/prelude-source.ts`,
analyzed the same way) runs before every script. The features it uses are needed
whatever scripts do:

- **Lockdown.** It walks every object reachable from `globalThis` and freezes it. It
  turns the overridable properties into accessor pairs (the "override mistake"
  workaround). This uses `Reflect.ownKeys`, `Object.getOwnPropertyDescriptor(s)`,
  `Object.defineProperty`, `Object.freeze`, `Object.getPrototypeOf`, getters and setters,
  `Set`, and `in`.
- **Hidden prototypes.** To reach the intrinsics only instances expose, it evaluates
  `function* () {}`, `async function () {}` and `async function* () {}`, plus
  `Int8Array`. It also reads the array, Map, Set and string iterators through
  `Symbol.iterator`, and the regexp-string iterator through `Symbol.matchAll`. Iterator
  helpers and `Iterator.from` are probed with `typeof` first; the others are not
  guarded.
- **The `tools` object** is a Proxy with a `get` trap that calls `Reflect.get`, so that
  misspelled tools give a helpful error. 2,329 of 2,535 scripts (92%) read
  `tools.something` and go through that trap.
- **Other:**
  - `Map` (namespaces);
  - `JSON.parse`/`stringify` (the host protocol);
  - `Promise`;
  - a regex lookahead;
  - rest parameters and spread;
  - `instanceof`, `typeof`;
  - `Error`, `TypeError`, `RangeError`.

pi's prelude puts a hard floor under these features:

- **Can't drop:** Proxy, Reflect, Symbol, getters/setters, generator and async-generator
  syntax, and the `Object` reflection functions.
- **Can't drop without a pi change:** typed arrays need `Int8Array` to exist (a one-line
  `typeof` guard in pi would lift that).

The lockdown also means that everything the engine ships is walked and frozen on every
fresh VM. djs saves an image after the prelude, so a start that reuses it skips this.
Fewer builtins still means a smaller per-session prelude image.

djs's own builtins (builtins/*.ts) are written in a subset without classes, generators,
closures or Proxy, so no engine-internal code depends on the user-facing features
discussed here.

## 4. Dynamic use a static scan can't see

- **Protocols behind common syntax.** The protocols run even where no script names them:
  - for-of and destructuring (765 scripts) run the iterator protocol:
    `Symbol.iterator`, %ArrayIteratorPrototype%, `next`/`done`/`value`;
  - every `await` (2,481 scripts) does a thenable check (`then` lookup) and runs the
    promise machinery;
  - template literals (998) and `String(x)` convert with ToPrimitive, which looks up
    `Symbol.toPrimitive`;
  - `JSON.stringify` and `text()` (1,267) look up `toJSON`;
  - regex String methods (136) go through `Symbol.replace` and its siblings.

  Well-known symbols and the iterator and promise machinery are therefore load-bearing,
  even with zero `Symbol` mentions.
- **Tool results are JSON.** The host passes arguments and results as JSON text (pi's
  protocol), so values from tools are plain objects, arrays, strings, numbers, booleans
  and null. No Date, Map, typed array, BigInt, Symbol, accessor or class instance can
  arrive from the host. Rejections are `Error` objects with copied own properties.
  Images are base64 strings inside plain objects.
- **Computed member access** (`o[k]`, 561 scripts) is mostly array indexing and record
  lookups. A static scan can't name the property; nothing suggests it reaches reflective
  features.
- **The frozen realm.** Writes like `err.name = …` on an Error subclass would go through
  the prelude's accessor setters. No script writes `.name`, `.message`, `.toString` or
  `.constructor`.

## 5. Candidates

Sizes:

- **Measured:** a variant build with `scripts/size/variant.sh` (the shipped build, with
  one change). The base module is 1,065,090 bytes, 460,886 gz.
- **Estimated "code":** the bytes of the functions whose names belong to the feature in a
  names build, at the module's gzip ratio of 0.38. This is a lower bound for code that
  is inlined elsewhere, such as the BigInt branches in `call_intrinsic` and
  `step_once`.
- **Estimated "image":** the feature's share of builtins/gen/prelude.js applied to the
  packed base image (56.8 KB).

| candidate | scripts | prelude | saving (gz) | where | recommendation |
| --- | ---: | --- | --- | --- | --- |
| `\p{…}` in regexps | 0 | no | **21.0 KB, measured** | cargo feature `regexp-unicode-props` (D1500); crates/djs-core/src/unicode/property.rs, props.rs | **Off for the pi module**; a SyntaxError names the cause |
| full Unicode normalization | 0 (`localeCompare`: 6) | no | **19.8 KB, measured** | cargo feature `unicode-normalization` (D1760); unicode/normalize.rs, norm_tables.rs | **Off for the pi module**; `normalize` then throws on strings that would need the tables, `localeCompare` compares code points |
| both | | | **40.7 KB (8.8%), measured**: 420.1 KB, 1.47x quickjs-wasi | | |
| typed arrays, ArrayBuffer, DataView (incl. Float16, base64/hex) | 0 | `Int8Array` must exist | ~13 KB (code ~5, image ~8.5) | builtins/typedarray.ts, dataview.ts, arraybuffer.ts; builtins/typed.rs, heap/typed.rs | Candidate for a cargo feature; needs a `typeof` guard in pi's prelude |
| BigInt | 0 | no | ~10 KB code (+ image ~0.4); a BigInt branch in every arithmetic, comparison and conversion op | crates/djs-core/src/bigint/, interp/run/bigint.rs, builtins/bigint.ts, lexer | Candidate for a cargo feature: literals become a SyntaxError, `BigInt` is absent; BigInt64Array goes with it |
| Iterator helpers, `Iterator.from` | 0 | guarded by `typeof` | ~5 KB image | builtins/iterator.ts | Candidate (ES2025; prelude already copes without) |
| decorators (TC39 and TS legacy) | 0 | no | ~5 KB (code ~3, image ~2) | compiler/decorators.rs, ts_meta.rs, builtins/decorators.ts | Candidate if codemode stays JavaScript; scripts can't be TypeScript today |
| `using`, DisposableStack, AsyncDisposableStack, SuppressedError | 0 | no | ~3 KB (code ~1, image ~2.4) | compiler/using.rs, builtins/disposable.ts | Candidate (newest syntax, never used) |
| async generators, for-await, `Array.fromAsync` | 0 | async-generator syntax | ~2.5 KB image | builtins/asyncgen.ts, fromasync.ts | Keep the syntax (prelude); `Array.fromAsync` alone ~1 KB |
| WeakMap, WeakSet, WeakRef, FinalizationRegistry | 0 | no | ~2 KB (code ~1, image ~1.2) | builtins/weak.ts, weakref.ts; heap/weakref.rs, ephemeron marking in heap/compactor.rs, cleanup jobs in interp/run/late.rs | Candidate for complexity rather than size: ephemerons and finalization in both collectors, and callback timing for determinism |
| eval, Function constructors | 0 | no | ~1 KB | compiler/dynamic.rs, builtins/eval.rs, eval.ts; D015 sloppy `this` | Low value: the compiler stays anyway. Dropping removes runtime code generation and the D015 special case |
| `escape`/`unescape`, URI functions | 0 | no | ~2 KB code | builtins/uri.rs, uri.ts | Keep URI functions (plausible with URLs); Annex B `escape`/`unescape` could go |
| unused `@fast` paths: trig/exp/log Math, `charCodeAt`/`codePointAt`/`substring`/case conversion | 0 | no | ~1–2 KB | builtins/fast.rs, `@fast` in math.ts/string.ts | Demote to the bytecode builtin if perf's benchmarks don't need them |
| classes (incl. fields, private names, static blocks, accessors) | 0 | no | ~7.5 KB code | compiler/class.rs, interp/run/class.rs | **Keep**: core syntax; models use classes in longer programs, and errors would be confusing |
| generators | 0 | syntax | ~1 KB code, ~1 KB image | compiler/generator.rs, builtins/generator.ts | **Keep**: needed by the prelude and shares the suspension machinery with async functions |
| Proxy, Reflect, Symbol, accessors, `Object` reflection | 0 | yes | | | **Keep**: pi's prelude |
| `arguments`, labels, `in`, `instanceof`, for-in, `delete`, `switch` | 0–4 | some | < 1 KB each | | Keep: small, core, test262 |
| sloppy mode, `with`, direct eval | 0 | no | already absent | ARCH non-goals | The data confirms the non-goal: no script tried them |
| Intl | 0 (`localeCompare` 6) | no | already absent | | The data confirms the non-goal |

The `@native` builtins (Array forEach/map/push/find/filter/reduce/sort, JSON
serialization, promise jobs) all serve common features: reduce 14 scripts, sort 36, find
27, map 694. None is a demotion candidate.

### What dropping would look like

- **Size budget.** The two Unicode features are a build switch away. They are the
  cheapest way to get under the D020 budget: 460.9 → 420.1 KB gz against a budget of
  ~430 KB.
- **Cargo features for the rest.** The other candidates would follow the same pattern: a
  cargo feature that leaves the builtins out of the image, plus a clear error. Dropped
  syntax would raise a SyntaxError that names the feature; dropped globals would simply
  be absent. That leaves the default build fully conformant for test262 and the
  differential corpus, while the pi module is slimmed.
- **What the next tier adds up to.** All cargo-feature candidates together
  (typed arrays, BigInt, Iterator helpers, decorators, `using`, weak references,
  eval/Function, escape) add roughly 40 KB gz more. This is an estimate; each needs a
  variant build to confirm.

## 6. Risks

- **The sample.**
  - It covers one user over five weeks, with one prompt and tool catalog style; other
    users, other models or data-heavy tasks may differ. The likeliest to appear
    elsewhere are typed arrays (binary file handling, image bytes), `\p{…}` (text
    processing in other scripts), `normalize`/`localeCompare` (sorting names), and
    classes in long scripts.
  - The analyzer can be re-run on any session set (section 7).
- **Hard failure instead of slowness.** A dropped feature fails the script, while qjs
  (pi's current engine) runs it. Codemode's model loop recovers from errors, but each
  costs a turn. A clear message ("BigInt is not supported in codemode") limits the cost.
- **pi's prelude.**
  - It uses Proxy, Reflect, accessors, `Object` reflection, Symbols, and generator and
    async-generator syntax. It also needs `Int8Array`.
  - Any cut there needs a coordinated pi change. Moving lockdown into the engine (a
    native "freeze the realm") would remove most of the prelude's demands.
- **test262 and the oracle.** Cut families take their tests with them:
  - typed arrays, ArrayBuffer and DataView: ~3,000 test files;
  - Iterator helpers: 654;
  - `\p{…}`: 613;
  - Proxy and Reflect: 464;
  - DisposableStack, `using` and SuppressedError: ~400;
  - weak references: ~300;
  - BigInt: 1,506 files carry the BigInt feature tag.

  The default build should stay complete, so the gate keeps testing everything. The
  slim build would need its own expected-failure list, or a test262 feature filter.
  Corpus cases that use a cut feature would need a skip directive for the slim build.
- **Suspension and snapshots.** Generators, async generators and the promise machinery
  share the resumable-frame code. Cutting user-facing generators saves little, because
  async functions need the same machinery.
- **User expectations.** Codemode presents itself as JavaScript, and models assume a modern
  engine. Missing builtins would be absent, so `typeof X === "undefined"` lets
  feature-testing code adapt. Missing syntax is a SyntaxError for the whole script.

## 7. Reproduce

The raw data stays in `work/scratch/0130/` (gitignored); scripts print counts only.

```sh
scripts/harvest/extract.py work/scratch/0130/extract.jsonl     # ~/.pi/agent/sessions
cargo build -p djs-cli
scripts/harvest/features.py work/scratch/0130/extract.jsonl work/scratch/0130/features.jsonl
# pi's prelude as a file: PRELUDE_SOURCE from pi-private's prelude-source.ts
scripts/harvest/features.py --summary work/scratch/0130/features.jsonl \
    --prelude work/scratch/0130/pi-prelude.js                     # the table, markdown
OUT=work/scratch/0130/variants scripts/size/variant.sh nonorm PATCH.py   # a size variant
```
