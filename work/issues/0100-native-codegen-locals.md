# 0100: Generated native builtins keep values in Rust locals (0081 P8) + more run_hot ops

status: done
agent: perf
branch: issue/0100-native-codegen-locals
depends-on: 0093
touches: crates/djs-bgen/src/emit_rust.rs (codegen), crates/djs-core/src/builtins/gen/natives.rs (regenerated), crates/djs-core/src/interp/run/native.rs, crates/djs-core/src/interp/run/hot.rs

## Goal
After 0093, plain bytecode is at 0.44–0.63x quickjs-wasi and calls ~1x; pi-like is ~1.5x,
and what's left there is the builtin share: generated natives still cost hundreds of
instructions per element (0081 F3: every local a checked arena slot with a NaN-box round
trip, fast-path results pushed then popped, out-of-line slot accesses in wasm, 6 state arms
per element). 0081 P8: keep values in Rust locals between resume points (they must be in
frame slots only across resume points, since GC and suspension happen only there, D1632).
Also the next run_hot candidates from 0093 (object, global-cache misses, concat).

## Acceptance criteria
- [x] bgen's Rust backend keeps locals and temporaries in Rust variables within a state
      arm and across arms that can't suspend or collect, spilling to frame slots only
      before resume points (calls that may run user code, allocation points that may
      collect: check the D1632 contract), with the D1700/D1900 retry rules unchanged.
- [x] Per-element instruction counts for the native builtins (perelem.py) and the engine
      track, wasm first; both backends identical; stress and determinism checks
      identical; code size per native (scripts/native-sizes.sh), which should go down.
- [x] The next run_hot ops if measurement supports them.
- [x] Coordinate with infra (bgen and the natives are its code) through the coordinator.

## Log
- Profiled first (wasm cpu profile of `a.map(x => x)`; native lldb trace of forEach). Per
  element the native's own arms were only part of the cost: the callback call
  (`n_call` -> `call_intrinsic` -> `Exec::call`, ~500 native instructions),
  `get_cap_chk`/`put_cap_chk` through `step_once`, and in wasm the out-of-line slot calls
  (`n_slot` 11%, `n_set_slot` 3%, `f64_of` 1.6% of the map profile).
- Every callback call is a resume point that ends the native's run, so "values in Rust
  locals between resume points" helps only within one element. Its wins come from the
  typed reads (loop counter, length, flags) and from natives that loop without leaving.
- Infra's D1900 hazard: registers hold only bool/i32/u32/f64, never heap references, so a
  collection in `n_retry` cannot leave one stale. Heap-typed locals stay in slots, and the
  retry contract is unchanged (D2000). The new test
  `native_registers_survive_collect_and_retry` checks it at caps where rounds fail
  without the retry (verified by disabling the retry: the test then fails at 2.7M/2.8M).
- Landed (D2000):
  1. bgen: typed locals as `Storage::Reg`: Rust variables, written through to their
     slots, loaded per entry state from a liveness pass. The state is a Rust local `st`.
  2. Inline call results go through `Exec::n_res` instead of a push/pop. A `Flow::Value`
     variant with a per-site `res` local did the same but cost 11 KB of wasm; dropped.
  3. `%Call` from natives calls `Exec::call` directly, skipping `call_intrinsic`'s
     dispatch.
  4. `Exec::call` reads the template fields once (`hot::template_head`, shared with
     run_hot's call) and writes the frame through `hot::write_frame` (also used by
     `push_native_frame`). That is −400 B of wasm in `Exec::call`.
  5. run_hot handles every box/capture op, including the `_chk` and `set_` forms; TDZ
     declines to `step`.
  6. The D1700 retry check uses a lookup table (`retry::retries`) instead of
     `INTRINSICS.contains` (~55 native instructions per `n_call`).
- Tried and dropped: a `native_step` branch in run_hot's decline path. It cost 2% on
  tight/loop/call1/prop1 in wasm (register pressure in the loop), and main's run_hot
  (0098) now returns into native callers anyway.
- Measured against main 6c29c519. Wasm instructions are min of 3 runs, spreads ≤1% unless
  noted; full suite in work/scratch/0100/wm-r1/r3/r4.txt.
  - higher-order 17,728 → 15,972 (−9.9%); calls 3,886 → 3,412 (−12%);
  - push-pop −6.0%; foreach −6.1%; json-replacer −3.2%; arrays −2.3%;
  - pi-like 10,548 → 10,323 (−2.1%);
  - forof +1.2%, json +0.6%; everything else within ±0.5%.
  - Native: forof, pushpop and tight are identical, so the wasm differences are V8 code
    layout.
- perelem (wasm, main → branch):
  - forEach 5,602 → 3,679, map 5,201 → 4,528, filter 5,542 → 4,858;
  - reduce 4,156 → 3,422, some 4,190 → 3,604, find 3,758 → 3,181;
  - sort 37,619 → 31,980;
  - join/slice/split ±2% (those are `@fast` now).
  - Native: forEach −23%, sort −12%, map −12%.
- Size: wasm 1,670,652 → 1,673,417 (+2.7 KB, all in the generated natives). The extra
  code is register copies across the state loop, so the acceptance criterion's "should
  go down" did not hold. `Exec::call` shrank 400 B.
- Guide: no page covers natives yet (introduction, values); none affected.

## Follow-ups
- run_hot candidates not done: `object`, global-cache misses, `concat`. Each needs a
  measurement against the D1352-style codegen sensitivity; the native_step branch shows
  that even a small arm can cost 2%.
- Natives' size: the register phis cost 2.7 KB. Typed temporaries (spilled across resume
  points) could be registers too; `n_slot`/`n_set_slot` stay out of line (D1634).
- From num (0104): `call_intrinsic` 37.5 KB, `step_once` 35.5 KB and
  `json_regexp_intrinsic` 14.3 KB of wasm. Move their cold arms out of line, in perf's
  interp/run area.
- The per-element floor for callback natives is now the call protocol: `Exec::call`, a
  frame push and a return into the native, one native run per element. A direct
  native→closure call path that shares run_hot's frame entry is the next lever.

## Result
Generated natives keep bool/i32/u32/f64 locals in Rust variables, with their slots
written through. Inline results skip the operand stack. Callback calls from natives skip
the intrinsic dispatch. `Exec::call` reads templates once, and run_hot runs all box and
capture ops. D2000 has the design.
- wasm: higher-order −10%, calls −12%, foreach/push-pop −6%, pi-like −2%; forEach per
  element −34%, sort −15%.
- Known gaps: natives +2.7 KB in wasm. The run_hot ops object, global misses and concat
  are left as follow-ups.
- New tools: `perelem.py` compares native and wasm builds side by side.
