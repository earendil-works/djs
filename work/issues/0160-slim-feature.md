# 0160: The slim codemode build as additive cargo features

status: done
agent: slim
branch: issue/0160-slim-feature
depends-on: 0149
guide: docs/guide/builtins.md (a section on the build variants), goals.md if relevant
touches: crates/djs-core/Cargo.toml + feature gates in builtins/ and the compiler (syntax for dropped families), crates/djs-prelude/ (base build per feature set), crates/djs-bgen/ (if gating needs it), crates/djs-wasm/Cargo.toml, scripts/build-wasm.sh (a --slim output), the gate (build + test the slim module), tests/builtins/registry.tsv (absent entries), TESTING.md

## Owner
"Slim codemode should also be an option; it should however just be a feature that can be
turned off (additive). Call it slim, and then a second feature, also on by default, for
unicode."

## Design (cargo features must be additive)
- The families docs/slim-codemode.md proposes to drop (typed arrays/ArrayBuffer/DataView,
  Iterator helpers + `Iterator`, DisposableStack/AsyncDisposableStack/SuppressedError +
  `using`, decorators, BigInt, WeakRef/FinalizationRegistry, optionally the newest
  proposals and Annex B leftovers) sit behind **one default-on feature**; building with it
  off gives the slim engine. Name: the owner said "call it slim"; since a feature that
  removes things isn't additive, name the default-on feature so that turning it off means
  slim (propose a name, e.g. `full`, in the issue log and tell the coordinator before
  renaming anything), and call the resulting build/module "slim" (`djs-slim.wasm`,
  `scripts/build-wasm.sh --slim`).
- A second default-on feature `unicode` replaces/umbrellas `regexp-unicode-props` and
  `unicode-normalization`.
- Dropped names give "not available in this build" errors (ReferenceError for globals,
  SyntaxError naming the feature for syntax), per 0149.
- The slim module is a gate artifact: built, size-tracked (its own ceiling), tested with the
  codemode corpora, the frozen-mode suite, pi's tests with the lockdown removed (scratch copy,
  as in 0146), and test262 filtered by feature tag.
- The registry marks absent builtins per build; the pre-created result shapes (D2067) follow
  the feature set.

## Log
- Feature names: proposed `full` + `unicode`; the owner confirmed `full` (D025).
- bgen writes gen/prelude-slim.js: files with a `// @slim-omit` line and builtins tagged
  `@slim-omit` are left out, as are kept installs onto omitted constructors; calls from kept
  builtins into omitted ones are listed in its header (all behind `IsTypedArray`).
  ToNumeric moved to conversion.ts (the interpreter needs it in every build).
- djs-core: `FULL` const guards (40 leaf arms, `Value::kind`/`as_bigint`, wire bytes),
  "not available in this build" errors (ReferenceError via `realm::ABSENT_GLOBALS`,
  SyntaxError for BigInt literals, decorators, `using`), realm leaves the families'
  prototypes undefined. Guarding `ObjectClass::from_u32` saved nothing: not in.
- Features forwarded through djs-prelude/-wasm/-difftest/-test262/-cli; djs-prelude asserts
  the base's build hash equals the library's (a build-dependency resolves features apart);
  umbrella `unicode` not in SOURCE_HASH.
- Registry: tests/builtins/registry-slim.txt (289 ids), `check_build`, resolver fallback so
  `Iterator.prototype` keeps its id (D2412).
- D2067 shapes: no chain belongs only to the omitted families; list unchanged.
- Gate (D2413): slim wasm build beside the size build; slim native unit tests
  (`vm::slim_tests` in both builds, frozen, registry), codemode corpora with stress, test262
  with tests/test262-slim.toml + expected-failures-slim.txt (120), pi's tests on
  djs-slim.wasm without lockdown (`pi-tests.ts --no-lockdown --expected`, 19 expected of
  226; 11 also fail on full), `slim_ceiling_gzip` 391,873.
- Sizes: full 455,468 gz (main 455,502); slim 390,849 gz (1.36x quickjs-wasi); base image
  250,544 B / 4,848 cells → slim 180,552 B / 3,495.
- Guide: docs/guide/builtins.md "Two builds: full and slim"; also TESTING.md, docs/host.md.

## Follow-ups
- Deeper slim cuts not done (est. a few KB gz): typed-array element paths and weak-reference
  handling in heap/ (other agents' area; `from_u32` guarding didn't help), decorator/`using`
  emitter code still linked behind the parser cut.
- pi-private: run djs-slim.wasm with `frozenBuiltins` and the prelude without `lockdown()`.
- The 0149 optional members (newest proposals, Annex B) stay in both builds; decide later.

## Result
`full` and `unicode` (default on) in djs-core and forwarded by every dependent crate;
`--no-default-features` is the slim build, `scripts/build-wasm.sh --slim` writes
target/djs-slim.wasm (390.8 KB gz vs 455.5 KB). Dropped names give "not available in this
build"; the registry lists the slim build's absent ids; the gate builds, size-checks and tests
the slim module (unit tests, codemode corpora, filtered test262, pi's tests without lockdown).
D2411–D2413.
