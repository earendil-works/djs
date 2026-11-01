# 0023: djs-wasm and the JS host package (Sandbox API)

status: in-progress
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
- [ ] `crates/djs-wasm`: the exports from ARCH ("Execution API"): `vm_new`, compile (once
      0014/0022 provide `Vm::compile`; until then, behind a stub that errors), `vm_call`,
      `vm_run(fuel) -> status`, host call inspection and `vm_resolve_host_call`,
      `vm_bytes`/`vm_from_bytes` (the snapshot precursor), staging-buffer allocation for
      inbound wire data, wire decode into a handle, value handles for outbound reads,
      and the arena base pointer and length (0021 follow-up). A small allocator (decide:
      the default `dlmalloc` in std-less wasm, or `talc`; record it) and a panic hook
      that traps cleanly. Built with the `wasm` profile.
- [ ] Only plain integer/pointer exports (no wasm-bindgen). Document the ABI in
      docs/host.md.
- [ ] `js/src/sandbox.ts`: load and instantiate (caching the compiled `WebAssembly.Module`
      per ARCH), `new Sandbox(opts)`, `compile`/`run` with a fuel slice loop, host
      function registration (sync host calls: `console.log`, `text`; async ones via the
      pending host call table if the VM supports it yet; otherwise sync only, and log the
      gap), values in via wire, values out via the reader (re-deriving views after every
      call), and snapshots (`bytes()`/`fromBytes()`).
- [ ] Node tests (`node --test`, in the gate via the existing js test step): instantiate,
      run hand-assembled or compiled code (whatever main supports when you get there),
      host calls, wire round trips through the VM, a snapshot round trip, OOM and a
      trap, which must give a clean error.
- [ ] Size track: tests/bench/size points at djs-wasm (or keep both). Record the size of
      the real cdylib in the Log.
- [ ] Startup number: `vm_new` + first `run` in node, recorded (ARCH target < 100 µs for
      `vm_new` from a base snapshot; there is no base snapshot yet, so record what it is).

## Notes
- 0022 (heap agent, in progress soon) adds `Vm::compile` and touches `vm/`. Do not
  modify `vm/` beyond tiny additive accessors; ask if you need more.
- Building wasm in the gate: the gate already builds djs-core for wasm32. Add the
  djs-wasm build there if it is fast (a debug build is fine for the gate).

## Log

## Follow-ups

## Result
