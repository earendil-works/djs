# 0023: djs-wasm and the JS host package (Sandbox API)

status: done
agent: num
branch: issue/0023-djs-wasm-host
depends-on: 0015, 0018, 0021
touches: crates/djs-wasm/, js/ (host API alongside wire.ts/reader.ts), tests/bench/size (point it at djs-wasm), docs/wire.md or a new docs/host.md

## Goal
ARCH.md "Host Interface" and phase P2 ("the wasm build, and the JS host package (wire in,
direct read out)"): a `wasm32-unknown-unknown` cdylib with a thin export layer over the
`Vm` API, and a TypeScript host package that loads it and exposes a small Sandbox API.
After this, djs runs in node through wasm, and its performance can be measured where it
matters.

## Acceptance criteria
- [x] `crates/djs-wasm`: the exports from ARCH ("Execution API"): `vm_new`, compile (once
      0014/0022 provide `Vm::compile`; until then, behind a stub that errors), `vm_call`,
      `vm_run(fuel) -> status`, host call inspection and `vm_resolve_host_call`,
      `vm_bytes`/`vm_from_bytes` (the snapshot precursor), staging-buffer allocation for
      inbound wire data, wire decode into a handle, value handles for outbound reads,
      and the arena base pointer and length (0021 follow-up). A small allocator (decide:
      the default `dlmalloc` in std-less wasm, or `talc`; record it) and a panic hook
      that traps cleanly. Built with the `wasm` profile.
- [x] Only plain integer/pointer exports (no wasm-bindgen). Document the ABI in
      docs/host.md.
- [x] `js/src/sandbox.ts`: load and instantiate (caching the compiled `WebAssembly.Module`
      per ARCH), `new Sandbox(opts)`, `compile`/`run` with a fuel slice loop, host
      function registration (sync host calls: `console.log`, `text`; async ones via the
      pending host call table if the VM supports it yet; otherwise sync only, and log the
      gap), values in via wire, values out via the reader (re-deriving views after every
      call), and snapshots (`bytes()`/`fromBytes()`).
- [x] Node tests (`node --test`, in the gate via the existing js test step): instantiate,
      run hand-assembled or compiled code (whatever main supports when you get there),
      host calls, wire round trips through the VM, a snapshot round trip, OOM and a
      trap, which must give a clean error.
- [x] Size track: tests/bench/size points at djs-wasm (or keep both). Record the size of
      the real cdylib in the Log.
- [x] Startup number: `vm_new` + first `run` in node, recorded (ARCH target < 100 µs for
      `vm_new` from a base snapshot; there is no base snapshot yet, so record what it is).

## Notes
- 0022 (heap agent, in progress soon) adds `Vm::compile` and touches `vm/`. Do not
  modify `vm/` beyond tiny additive accessors; ask if you need more.
- Building wasm in the gate: the gate already builds djs-core for wasm32. Add the
  djs-wasm build there if it is fast (a debug build is fine for the gate).

## Log
- `crates/djs-wasm`: cdylib + rlib, std on wasm32 with the built-in dlmalloc (D1231).
  Exports:
  - VM lifecycle, the arena pointer and length (= the image), collect;
  - compile (via `compiler::compile` + `Vm::instantiate`; 0022's `Vm::compile` was not on
    main yet);
  - global/object/host function/define, wire decode, JSON parse/stringify, errors;
  - handle bits/free/display, call/run/result, host call inspect/resolve, panic message.

  Seven native tests call the exports directly.
- ABI (D1230): i32 only, no imports, staging/out buffers, numbered VMs and handles,
  version 1. Hot paths decode straight from staging; `vm_from_bytes` takes the staging Vec
  over as the arena.
- `js/src/sandbox.ts`:
  - `loadModule` (cached per location); `Sandbox.create`, the synchronous
    `new Sandbox(opts, snapshot?)` and `fromBytes`;
  - `run` (fuel-slice loop, sync host functions) and `runAsync` (awaits promises from host
    functions while the VM is parked);
  - console methods as host functions; `functions`, `defineFunction`, `setGlobal`;
  - snapshots: a "DJSS" header with the host function names, then the image;
  - `SandboxError` kinds; traps read `djs_panic_*` and kill only that sandbox.

  11 node tests in `js/test/sandbox.test.ts`. The gate builds a debug djs-wasm and sets
  `DJS_WASM`.
- `text` from the criteria: nothing specific was needed; any host function registers
  through `functions` / `defineFunction`.
- Size: the size track now builds both the probe (old names, so history stays comparable)
  and djs-wasm (`size/djs-wasm ...`). djs-wasm at opt-level s: 459 KB file, 359 KB
  without custom sections, 132 KB gzip -9 stripped (at z: 437 / 298 / 116 KB).
- Startup (node 25, M-series): instantiate 31 µs, `vm_new` 11 µs (no base snapshot),
  `Sandbox.create` 48 µs, create + first `run("return 1")` 59 µs.
- The wasm boundary benchmarks deferred from 0018/0021 (`js/bench/wasm.bench.ts`) are in
  docs/host.md. Wire beats JSON in both directions, by more than natively.
- Finding: opt-level z is ~3x slower than s in wasm, because the arena accessors are not
  inlined. The coordinator approved switching `[profile.wasm]` to s (D1232, with the table).
- `try` and most builtins were not on main yet. The tests stick to operators, objects and
  host calls.

## Follow-ups
- When 0022 lands: switch `vm_compile` from `compiler::compile` + `instantiate` to
  `Vm::compile` (one function in crates/djs-wasm/src/lib.rs).
- `#[inline(always)]` on the hot arena accessors (`Arena::read/write::<T>`,
  `layout::read_array`, ...), then re-measure at z and s (D1232). If z gets close, it may
  come back for size.
- Install `wasm-opt` and measure the 400 KB budget after `wasm-opt -Oz`; the size track
  already reports it when the tool is present.
- `Vm::abort` (drop the in-flight call), so a sandbox that ran out of `maxFuel` stays
  usable. Today it is marked dead.
- Concurrent host calls (`Promise.all` over host functions), once the VM has async
  functions and a pending call table. The ABI already carries call ids.
- A base snapshot for `vm_new` (ARCH). `vm_new` is already 11 µs without one, so it is a
  low priority.
- djs-core has an `unused import: flags as opflags` warning in interp/run.rs, only on
  wasm32 builds (not mine; the gate does not run clippy on wasm32).

## Result
djs runs in node through wasm.
- Build: `scripts/build-wasm.sh` (the `wasm` profile, now opt-level s).
- Use: `import { Sandbox } from "@djs/host"` (js/src/sandbox.ts): `await Sandbox.create({
  functions, console, memoryLimit, fuelSlice, maxFuel })`, `run(code, params)` /
  `runAsync`, `bytes()` / `Sandbox.fromBytes`.
- Docs: docs/host.md has the ABI table, the Sandbox API, startup and boundary numbers, and
  the gaps. Decisions D1230 (ABI), D1231 (std + dlmalloc), D1232 (opt-level s).
- Benches: `node js/bench/sandbox.bench.ts` (startup) and `node js/bench/wasm.bench.ts
  <dir>` (the boundary in wasm).
- Known gaps:
  - compile goes through `compiler::compile` until `Vm::compile` exists;
  - one outstanding host call at a time;
  - running out of `maxFuel` kills the sandbox;
  - no base snapshot;
  - the JS that runs is limited by the compiler and builtins on main.
