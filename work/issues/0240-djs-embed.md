# 0240: djs-embed: the native embedding crate (E1–E7)

status: in-progress
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
- [ ] crates/djs-embed with the API above, documented; no djs-core types in its public API
- [ ] wire: Rust encoder, `Vm` decode without intrinsics, outbound encoder, UTF-8 tag without
      units; docs/wire.md updated; wire tests and wire_cross green
- [ ] uncatchable heap limit option in djs-core, tested (a `try/catch` and a `finally` do not run)
- [ ] snapshot `Send + Sync` (compile-time assertion), instantiate and reset measured
- [ ] async entry point with settled value; async and sync host calls; typed args/results
- [ ] tests for every failure mode, panics poison only their instance
- [ ] docs/guide page; `cargo run -q -p djs-docs -- check` passes
- [ ] the shipped wasm module's size within its ceiling (core additions should cost it nothing
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

## Follow-ups

## Result
