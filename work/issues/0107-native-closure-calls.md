# 0107: Direct native-to-closure calls; cold arms out of line

status: done
agent: perf
branch: issue/0107-native-closure-calls
depends-on: 0100
guide: - (the interpreter chapter, once docs writes it: tell the coordinator what changed)
touches: crates/djs-core/src/interp/run/ (native.rs, hot.rs, call paths, intrinsics dispatch), crates/djs-bgen/src/emit_rust.rs (only if the call sites change), crates/djs-core/src/builtins/gen/ (regenerated)

## Goal
0100's findings: every callback call from a native builtin ends the native's run, so the
call protocol (Exec::call, frame push, return into the native) is what's left of the
callback builtins' gap (pi-like ~1.5x quickjs-wasi). Build a direct native-to-closure call
path that shares run_hot's frame entry and returns straight into the native's resume state.
Also num's 0104 size suggestion for this area: move the cold arms of call_intrinsic (37.5 KB),
step_once (35.5 KB) and json_regexp_intrinsic (14.3 KB) out of line, and the remaining
run_hot candidates (object, global-cache misses, concat) where they measure well.

## Acceptance criteria
- [x] Per-element and engine-track numbers, wasm first (the callback workloads:
      forEach/map/filter/sort/reduce/find, higher-order, pi-like); invisible to images,
      fuel, stress and determinism checks (hot_loop_is_invisible extended).
- [x] Size before/after; the cold-arm moves must not cost speed.

## Log
- Profile (native lldb trace, forEach, ~1,480 instructions per element on main):
  - the callback call: `Exec::call` 175 + `n_call_once` 104 + `n_call` 22;
  - `run_hot` 400: the callback body, its return and the re-entry;
  - the native's arms 300; HasProperty/Get 140; `step_once` + `native_step` 60.
  - In wasm, the out-of-line accessors (`n_slot`, `n_arg`, `tail_capacity`,
    `object_class`) add a call each.
- Landed (D2060):
  1. `n_call_closure`: plain closures get their frame written in place, about −150
     native instructions per callback call.
  2. `tail_capacity` `#[inline]` (heap/layout.rs: one attribute outside my area; the
     coordinator was told). `hot::object_slot` without the division.
  3. `Exec::is_class` for the class checks of the native fast paths.
  4. Inline `AsNumber`/`AsFn`/`AsStr` in natives (bgen `inline_leaf`). Sort ran
     AsNumber through `call_intrinsic` 718k times in higher-order.
  5. Size: cold `type_error`/`bad_argument` faults, and the well-known symbol table.
- instr.py works again: it counts the ops `run_hot` declines, per opcode, and frames.
- Run_hot candidates (object, global misses, concat): instr.py on pi-like counts only
  209k declined bytecodes in total (36k `object`, 18k `to_string`, 9k `concat`). At ~30
  extra instructions each that is under 0.1% of pi-like, so I didn't add them. pi-like's
  time is in builtins: string index_of, alloc_init, JSON, GC, flatten.
- Cold arms of call_intrinsic/step_once/json_regexp_intrinsic: the DWARF line tables
  show the size is diffuse. Inlined Value tag tests (8.3 KB of call_intrinsic),
  arena get/set (7.8 KB of step_once), and Result plumbing are spread over ~200 arms,
  with no big cold arms. Moving arms out of line would only move bytes. Only the
  repeated `String::from`/`format!` of the error constructors and the byte-string
  `match` were real repetition (−1.3 KB).
- Tried and dropped:
  - a direct `native_step` in run_hot's decline arm: +2% on tight loops again;
  - `object_class` `#[inline]`: +8.8 KB;
  - batched argument reads (`n_srcs`): +13% per element and +1 KB.
- Measured against main 61889d96. Wasm instructions are min of 3 runs, spreads ≤1.6%;
  full table in work/scratch/0107/wm-c6.txt.
  - higher-order 15,929 → 13,852 (−13.0%); foreach −12.8%; forof −11.4%;
    arrays −8.1%; pushpop −7.6%; push-pop −6.4%; calls −4.3%;
  - call1 −3.4%; json-replacer −3.4%; pi-like 10,306 → 10,004 (−2.9%); loop −2.8%;
    tight −2.8%; strbuild −2.1%;
  - everything else −1.8% to +0.1%.
- perelem (wasm, main → branch):
  - forEach 3,694 → 3,077 (−17%), map 4,519 → 3,943 (−13%), filter 4,885 → 4,224
    (−14%);
  - reduce 3,389 → 2,913 (−14%), some −16%, find −15%;
  - sort 32,036 → 28,269 (−12%);
  - `[...a]` −7%, for-of −7%.
  - Native: forEach −9%, sort −11%, map −8%.
- Size: wasm 1,674,672 → 1,675,486 (+814 B).
  - `n_call` +988 (its copy of the frame writer), `tail_capacity` inlining +435;
  - error constructors and the symbol table −1.3 KB.
- Guide: no interpreter chapter yet; the docs notes are in the Result.

## Follow-ups
- The rest of a callback call is fixed costs:
  - the native's own state arms (~300 native instructions per forEach element: dispatch
    and slot reads);
  - HasProperty + Get reading the same dense element twice;
  - the callback body in run_hot.
  - The next levers are in bgen: fewer states per element, e.g. a fused
    "HasProperty+Get of a dense element" helper.
- heap: `object_class` is out of line in wasm and used at 176 sites. Inline class tests
  at the hot call sites, as `Exec::is_class` does, beat inlining it everywhere (+8.8 KB).
- pi-like's gap is in the builtins library now: string index_of, arena alloc_init, JSON
  parse/quote, flatten, GC. Those are other areas.

## Result
Plain closures called from natives get their frame written in place (D2060). On top
of that: `tail_capacity` inline, in-place class checks, and inline `AsNumber`/`AsFn`/
`AsStr` in natives.
- wasm: higher-order −13%, foreach −13%, forof −11%, arrays −8%, pi-like −3%, tight
  −3%; nothing slower. forEach per element −17%, sort −12%.
- Size +814 B net.
- `hot_loop_is_invisible` covers callback natives and compares both call paths.

For the docs agent (interpreter chapter, not written yet). What changed in how calls
and returns between natives and closures work:
- A native's `%Call` of a plain closure writes the callee's frame directly
  (`n_call_closure`), returns `Flow::Leave`, and its run ends.
- The closure's `return` in `run_hot` restores the native caller with `code_len` 0, so
  the next round runs `native_step` from the resume state.
- One native run per callback call, frames as before (D1631/D1632). No new resume
  points or contracts.
