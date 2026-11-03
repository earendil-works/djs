# 0160: The slim codemode build as additive cargo features

status: in-progress
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

## Follow-ups

## Result
