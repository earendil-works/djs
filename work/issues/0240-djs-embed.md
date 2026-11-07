# 0240: djs-embed: the native embedding crate (E1–E7)

status: done
agent: embed
branch: issue/0240-djs-embed
depends-on: -
touches: crates/djs-embed (new), crates/djs-core/src/wire (Rust encoder, outbound encoder, a UTF-8 tag without units), crates/djs-core/src/vm/mod.rs (small embedder API additions), crates/djs-core/src/interp (uncatchable heap limit), docs/wire.md, docs/host.md, js/src/wire.ts (only if the new tag is used there), docs/guide (a new page)
guide: a new page docs/guide/rust-api.md ("The Rust interface", under "Using djs"), docs/guide/limits.md (uncatchable heap limit), docs/guide/host.md / js-api.md where they describe the wire format

## Goal

D3170. Gondolin (`~/Development/gondolin`, read-only) wants to run JS network hooks inside its
Rust daemon with djs. Its research is `~/Development/gondolin/native/docs/djs-hooks.md`: read
§1 (embedding API), §2 (hook contract), §4 (byte passing and costs) and the requirement list
E1–E7. Its throwaway prototype `~/Development/gondolin/native/experiments/djs-hooks/`
(`src/lib.rs`, `src/shim.js`, `src/bin/bench.rs`) drives `djs_core::vm::Vm` directly today:
that is the usage the new crate must make unnecessary.

Build `crates/djs-embed`, a std crate with a stable API that exposes no djs-core type:

- **E1** semver'd crate (own `version = "0.1.0"`, `publish = false` for now), deps only djs-core
  and djs-prelude (no third-party runtime crates; dev-dependencies too should stay std-only if
  possible). Module docs that are the API reference.
- **E2** `Snapshot` (name as you see fit): compiled once from the host's setup (host function
  declarations, prelude/shim scripts, the policy script; frozen builtins option), `Send + Sync`,
  cheap to clone (Arc). `instantiate` makes a fresh VM; an instance can `reset` to the snapshot
  reusing its allocation (`Vm::replace_image` or better). Measure both against the prototype's
  ~6–7 µs `from_bytes_trusted(image.clone())`.
- **E3** host functions declared by id and name, sync or async; the host reads arguments typed
  (`&str`/`String`, `&[u8]` borrowed from the arena where possible, f64, bool, plain data as an
  owned value tree, or wire bytes) and answers typed (undefined/null/bool/number/string/bytes/
  plain data/pre-encoded wire, or an error of a kind with a message).
- **E4** an async entry point: calling a function (by global path) that returns a promise ends
  with the promise's settled value (fulfilled or rejected), kept GC-safe across runs (a handle),
  without the prototype's `__settle` trick. Sync results work the same way.
- **E5** plain data without JSON:
  - a Rust encoder of the inbound wire format (docs/wire.md) in djs-core's `wire` module
    (no_std, usable by any Rust host) and re-exported or wrapped by djs-embed;
  - decoding needs no `Intrinsic` from the host (a `Vm` method);
  - an outbound encoder, VM value → wire bytes / value tree, for plain data (primitives,
    strings, dense arrays with holes, plain objects with own enumerable data properties,
    Uint8Array); it never runs JS (refuses accessors, proxies, class instances with a clear
    error naming the path, like js/src/wire.ts does);
  - a UTF-8 string form without the UTF-16 length up front (the decoder counts while it
    validates); keep the format versioned and documented in docs/wire.md, and the cross-language
    test (`wire_cross`) green;
  - strings and bytes the host returns go straight into the heap (one copy, ASCII memcpy fast
    path), not through an intermediate wire buffer. Wrapping host-owned bytes without a copy is
    not possible with the single arena (ARCH "The Core Idea"); say so in the docs.
- **E6** budgets a script cannot catch or evade: per-call fuel (sliced; the host regains control
  between slices), the memory cap, a wall-clock deadline checked between slices, and an option in
  djs-core that makes reaching the heap cap end the run uncatchably (no `catch`/`finally` runs;
  the call fails; the VM is idle and usable afterwards). djs-embed turns it on. Document what
  `Vm::abort` guarantees.
- **E7** one `Error` enum; a documented failure model: every outcome that is not a settled
  value is an error (throw, rejection, fuel, deadline, memory, never-settles, bad host use,
  panic). Every djs-core entry that runs engine code is wrapped in `catch_unwind`; a panic
  poisons that instance (later use returns an error; the host drops it), never the snapshot or
  other instances. Check and document that djs-core has no global mutable state outside tests.

Also: a guide page for Rust hosts, and unit/integration tests in djs-embed (`tests/it/main.rs`
layout, D2791) that cover each requirement, including the fail-closed cases the prototype's bench
prints (fuel, memory, bad return, never settles, syntax error, panic via a test hook if one
exists). Add a gate leg if djs-embed's tests are not picked up by the existing workspace test
leg (check scripts/gate.py; clippy legs too).

## Acceptance criteria
- [x] crates/djs-embed with the API above, documented; no djs-core types in its public API
- [x] wire: Rust encoder, `Vm` decode without intrinsics, outbound encoder, UTF-8 tag without
      units; docs/wire.md updated; wire tests and wire_cross green
- [x] uncatchable heap limit option in djs-core, tested (a `try/catch` and a `finally` do not run)
- [x] snapshot `Send + Sync` (compile-time assertion), instantiate and reset measured
- [x] async entry point with settled value; async and sync host calls; typed args/results
- [x] tests for every failure mode, panics poison only their instance
- [x] docs/guide page; `cargo run -q -p djs-docs -- check` passes
- [x] the shipped wasm module's size within its ceiling (core additions should cost it nothing
      or near nothing: keep new core code out of the wasm module's reachable set, or measure)

## Notes
- The prototype's numbers to compare with (Apple M5 Max): empty hook 3.5 µs, header policy
  26 µs pooled / 33 µs fresh, 64 KiB body 20 µs pooled / 35 µs fresh. The bench port is 0247;
  a quick micro-bench of instantiate/reset/call here is enough.
- ES modules come in 0241/0243; leave room in the API (a `Source` with a kind) but do not build
  module support here.
- Async host calls: `Vm::new_async_host_function`, `new_host_calls`, `resolve_host_call`; the
  host inbox applies results in order (D1670). Sync: `Status::HostCall`.
- Keep djs-core's additions minimal and in the style of vm/mod.rs; anything that would land in
  the shipped wasm module must be measured (TESTING.md "Size budget").

## Log

- 2026-11-07 embed: baseline shipped module 426,792 B gzip, ceiling 426,806: 14 B of
  slack, so nothing new may reach djs-wasm. New djs-core feature `embed` (code only, in
  djs-core's defaults, off in djs-wasm; build.rs/compat_inputs classify it image-neutral)
  holds what is reachable from the wasm paths: the decoder's 0x12 arm, the
  `fatal_out_of_memory` check, `Vm::reset_to`, `wire::encode`.
- djs-core commits: `wire::Writer` (Rust encoder, src/wire/writer.rs), tag 0x12
  `STR_UTF8_UNCOUNTED` (units counted from lead bytes, then the existing validating
  transcode), `wire::encode` (src/wire/encode.rs, heap → wire, explicit stack, path errors,
  `max_bytes` cap against sharing blowup), `Vm::decode_wire`/`encode_wire`,
  `ExecOptions::fatal_out_of_memory` (routes the cap's OOM through `host_out_of_memory`:
  status `HostOutOfMemory`, `after_run` aborts), `Vm::reset_to(&[u8])` (copies into the old
  arena Vec). Tests: wire::tests writer/uncounted/outbound, vm::tests
  fatal_out_of_memory_runs_no_handler, reset_to_reuses_the_arena.
- Clippy lints the build-dependency copy of djs-core (djs-prelude's build script, no
  default features) too: helpers used only by `embed` code need the cfg as well.

- Size: the first build cost +116 B gzip at about the same raw size. Cause, found with
  `--keep-names` builds of main and the branch and the function order (not sizes): the
  module's function order follows first references, so an unused `Vm::decode_wire` moved
  `wire::decode`, and `transcode`'s closures moved with a new enclosing fn (`utf8`). Fixed by
  gating `decode_wire` and the `ExecOptions` field behind `embed` and keeping `string()`'s
  structure (the uncounted count is a peek in a separate fn). Now the shipped module is
  426,792 gzip / 980,158 raw, identical to main; slim 360,471 (ceiling 360,485). A
  comment-only change moves gzip by ~3 B (source hash).
- `Vm` was not `Send` with djs-core's `dump` (Rc in the dump caches), and workspace builds
  unify `dump` in: renamed Rc -> Arc in vm/dump*.rs and vm/mod.rs (outside `touches:`, a
  mechanical rename; D3183).
- djs-embed (crates/djs-embed): src/{lib,snapshot,instance,value,data,error}.rs, unit tests
  in src/tests.rs (panic poisoning through the guard, and a real panic from a corrupted
  heap), integration tests tests/it/{snapshot,calls,data,failures,example,bench}.rs. The
  workspace `test` and `clippy` legs pick it up (no new gate leg); doctest = false, the docs'
  example is an `ignore` block that tests/it/example.rs runs.
- Bench (release, load ~140): image 192 KiB; instantiate 2.9-3.9 µs, reset 2.8-3.0 µs, empty
  call 0.2 µs, header policy pooled 12.5-13.5 µs, reset + call / fresh + call 17-18 µs.
- Decisions D3180-D3184. Guide: new docs/guide/rust-api.md (SUMMARY after js-api), limits.md
  (the final cap option), host.md (Rust encoder, uncounted strings); js-api.md unaffected.
  docs/wire.md and docs/host.md updated.

## Follow-ups
- Errors a host function throws (`Throw`) have no `stack`: `Vm::new_error` records none. A
  stack of the calling frames would help policy authors.
- `Step::Wait` reports async calls only when the VM waits; a long CPU-bound script that
  issues one early starts it late (at most a slice). A `Step` that reports new calls at once
  would need a fifth variant.
- ARCH.md does not mention djs-embed or the `embed` feature yet (coordinator).
- Instances keep the heap capacity a call grew them to across resets; a host pool may want
  a way to shrink (`Arena::shrink_capacity` exists).

## Result
crates/djs-embed 0.1.0 (publish = false), deps djs-core and djs-prelude only. API:
`Snapshot::builder()` (`host_fn(HostFn::sync|asynchronous)`, `global(name, Data)`,
`source(Source::script)`, `frozen_builtins` (default on), `memory`, `fuel`, `build`);
`Snapshot` (Arc, Send + Sync asserted) `instantiate(Limits)`; `Instance` (Send asserted)
`start(path, args)`, `step() -> Step::{Done, HostCall, Wait(ids), Yield}`, `host_call(id)`,
`resolve(id, Result<Value, Throw>)`, `result() -> ValueRef`, `call(path, args, closure)`,
`reset`, `abort`, `set_now`, `deadline`, `fuel_used`, `set_limits`; values in `Value<'a>`
(Str/Bytes/Data/Wire/...), out `ValueRef<'a>` (`as_str` Cow borrowed for flat ASCII,
`as_bytes` borrowed, `to_data`, `to_wire`), `Data` tree with `to_wire`/`from_wire`; one
`Error` enum (Syntax, Exception, OutOfFuel, Deadline, OutOfMemory, NeverSettles, Data,
Usage, Panicked, Poisoned). The crate docs in src/lib.rs are the reference; guide page
docs/guide/rust-api.md.

djs-core (feature `embed`, default on, off in djs-wasm): `wire::Writer` (Rust encoder),
tag 0x12 uncounted UTF-8, `wire::encode` / `Vm::encode_wire` (outbound, path errors, byte
cap), `Vm::decode_wire`, `ExecOptions::fatal_out_of_memory`, `Vm::reset_to`; `Vm` is `Send`
in every build. Shipped wasm module unchanged in size (426,792 gzip).

Known gaps: see Follow-ups. Modules (0243), the bench port (0247) and musl (0246) build on it.
