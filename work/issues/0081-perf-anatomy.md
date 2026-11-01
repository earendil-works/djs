# 0081: Why is djs an order of magnitude slower than QuickJS? (disassembly-level investigation)

status: done
agent: perf
branch: issue/0081-perf-anatomy
depends-on: -
touches: work/issues/0081-perf-anatomy.md (the report), work/scratch/0081/, scripts/ or crates/djs-bench/ (new measurement tools only). No engine changes.

## Goal
On real workloads djs is still roughly an order of magnitude slower than QuickJS (pi-like
codemode script ~4–5x, for-of 10–15x, push/pop 6x, indexOf 7x, after 0035/0062/0063). The
owner wants to understand **why**, at the machine-code level. QuickJS is a reference
point, not the goal (it is itself far from optimal), so the question is "what does djs
spend instructions and memory traffic on that a well-designed interpreter would not", not
"how do we copy qjs".

## What to deliver
- [ ] Profiles with attribution down to instructions: macOS `sample`/`xctrace` (no
      installs; use what is on the machine; `otool -tV` / `objdump` from Xcode,
      llvm-tools only if already installed) on a native release build with symbols
      (`debug = 1` is already in the release profile), and the wasm build in node
      (`--cpu-prof`, plus inspecting the wasm code for the hot functions). Workloads: a
      tight loop, calls, property access, a for-of loop, push/pop, string building, a
      pi-like codemode script, a JSON-heavy script.
- [ ] Disassembly of the hot paths: the dispatch loop (`Exec::run`, `fast_step`),
      instruction decode (immediates, bounds checks), the value stack and frame access
      (where `pc`, `sp`, `fuel`, `bp` live: registers or memory, re-derivation after calls
      or GC, D1350 notes), NaN-boxing tag checks, arena reads and writes (bounds checks,
      D1260), call/return (frame push, argument copying, header writes), the
      `call_intrinsic` dispatch, native frames (0063), allocation, and the safepoint and
      fuel checks. Count instructions and memory ops per bytecode for common ops, and per
      JS-level operation (for example one `arr.push(x)`).
- [ ] Root causes, ranked by impact with evidence: for example dispatch design (a match
      on a u8 with bounds checks vs threaded code), state kept in memory instead of
      registers, Result/Fault plumbing on hot paths, heap bounds checks, generic arena
      accessors, allocation per operation (iterator results, arguments arrays,
      closures), the builtin-language layering (how many intrinsic calls per builtin),
      fuel accounting, and so on.
- [ ] Proposals per cause: the expected gain, the effort, and the risk to the
      architecture's guarantees (position-independent heap, suspend anywhere,
      deterministic images, no unsafe without a decision). Separate cheap fixes from
      structural redesigns, and say what the ceiling looks like for this design.

## Notes
- The machine is shared and loaded: profile with care, repeat, and report variance.
  Don't record benchmarks (bench.sh --record) under load.
- Own worktree.

## Log

- 2026-11-01: Method. All numbers are **machine instructions retired** (and cycles) per
  process from `/usr/bin/time -l`. On this loaded machine (load 15-40) instruction counts
  repeat to <0.5% (spread columns below), cycles to 2-28%, wall time not at all (the
  same wasm run took 453 ms and 1094 ms an hour apart), so conclusions rest on
  instructions, checked against cycles. Builds: native release with LTO, one codegen
  unit, `debug = 1` (D1620's note: default builds vary ±15% from layout); wasm is the
  `wasm` profile (opt-level s, LTO) with the name section kept (`STRIP=debuginfo`).
  Native data is from 6c7823a, wasm data from 32b9f05 (main after 0083's 16-byte
  entries); D016 arrived mid-investigation and made wasm the primary target, so every
  important number below exists for wasm.
- Tools (scripts/perf/, README there; outputs in work/scratch/0081):
  - `measure.py` / `wmeasure.py`: whole workloads, djs vs qjs natively and djs-wasm vs
    quickjs-wasi (pi's engine, imported read-only from pi-private) in node; instructions
    of the node process minus an empty run.
  - `micro.py` / `wmicro.py`: instructions per JS-level operation (a loop body repeated
    10x minus 0x). `perelem.py`: instructions per array element of iteration builtins.
  - `trace.py` + `trace_report.py`: an **exact dynamic instruction trace** of the native
    build through lldb single-stepping (300 steps/s), segmented at the run loop's fuel
    check, so every bytecode (and every native-builtin transition) gets its exact
    instruction, load, store and branch count and a category from the inlined source line.
  - `instr.py`: an instrumented *copy* of the engine (never the worktree) counting
    bytecodes per op/function (fast vs slow path, user vs prelude), intrinsic calls,
    native state arms, frames, allocations by cell kind and collections.
  - `prof.py` (samply, self time by inlined function/line, memcpy/malloc by caller),
    `cpuprof.py` (node `--cpu-prof` of the wasm build), `wasm_arm.py` (reads one
    `fast_step` arm out of `wasm-objdump -d`).
  - `nobounds.py`: an experiment copy with unchecked arena accesses (measures the checks).
  - `model/`: a 400-line dispatch-cost model of the same loop in three styles.
- No engine changes on the branch.

## Findings

### F0. The headline: in wasm the gap is the builtins, not the interpreter

wasm, djs-wasm vs quickjs-wasi in node 25, pi's fuel slicing (10,000 fuel per `vm_run`,
js/pi/vm.ts). Instructions minus an empty run, min of 3 interleaved runs (spread
≤ 2.4%), cycles are noisier (≤ 17%):

| workload | djs Minstr | qjs-wasi Minstr | instr ratio | cycle ratio | djs wasm memory |
| --- | ---: | ---: | ---: | ---: | ---: |
| tight (`for … s += i`) | 14,086 | 15,659 | 0.90 | 0.83 | 2.8 MiB |
| loop (engine/loop) | 13,262 | 13,227 | 1.00 | 0.89 | 2.8 |
| call1 (`s += f(i)`) | 7,731 | 7,050 | 1.10 | 0.92 | 2.8 |
| calls (engine) | 6,044 | 4,057 | 1.49 | 1.09 | 2.8 |
| prop1 (`s += o.c`) | 4,953 | 5,504 | 0.90 | 0.69 | 2.8 |
| props (engine) | 10,594 | 9,455 | 1.12 | 0.89 | 5.2 |
| arrays (engine) | 27,906 | 16,882 | 1.65 | 1.64 | 30.4 |
| strings (engine) | 8,619 | 6,005 | 1.44 | 1.56 | 20.9 |
| strbuild (`+=`, join) | 5,835 | 3,200 | 1.82 | 1.61 | 57.9 |
| push-pop (engine) | 9,235 | 3,084 | 2.99 | 2.54 | 5.2 |
| pushpop (1M push+pop) | 14,656 | 4,739 | 3.09 | 2.70 | 2.8 |
| higher-order (engine) | 18,635 | 4,784 | 3.90 | 3.07 | 49.3 |
| **pi-like** (codemode, 0062) | 26,007 | 7,268 | **3.58** | **3.09** | 7.1 |
| indexof (1000 elements) | 3,761 | 574 | 6.55 | 3.45 | 2.8 |
| **forof** (4M elements) | 43,697 | 7,176 | **6.09** | **5.72** | 22.8 |
| json (engine) | 1,349 | 2,193 | 0.62 | 0.63 | 21.1 |
| jsonh (JSON + read loop) | 2,669 | 3,571 | 0.75 | 0.74 | 20.8 |

Per JS operation in wasm (`wmicro.py`, machine instructions per op, min of 2):

| operation | djs-wasm | qjs-wasi | ratio |
| --- | ---: | ---: | ---: |
| local read `x;` / write `x = i` | 186 / 285 | 212 / 431 | 0.9 / 0.7 |
| own field read `o.c` / write | 372 / 643 | 447 / 468 | 0.8 / 1.4 |
| call+return `f(i)` / method `o.m(i)` | 1,211 / 1,370 | 966 / 1,226 | 1.3 / 1.1 |
| `{a, b}` / `[i, i]` / closure | 3,500 / 2,067 / 4,132 | 2,822 / 1,623 / 3,207 | 1.2 / 1.3 / 1.3 |
| `'a' + i` / template | 4,086 / 4,431 | 1,825 / 2,638 | 2.2 / 1.7 |
| `push`+`pop` | 12,418 | 3,011 | 4.1 |
| `a.indexOf(3)` (len 8) | 8,009 | 1,868 | 4.3 |
| `for (const x of [1,2,3])` | 47,865 | 7,526 | 6.4 |
| `a.map(x => x)` (len 4) | 69,216 | 12,089 | 5.7 |
| `s.startsWith` / `s.trim()` / `s.slice` | 10,239 / 9,815 / 9,488 | 1,788 / 1,914 / 2,025 | 5.7 / 5.1 / 4.7 |
| `o.hasOwnProperty('a')` | 11,067 | 1,689 | 6.6 |
| `'a,b,c'.split(',')` | 41,853 | 6,214 | 6.7 |
| `line.includes('throw')` | 23,459 | 3,183 | 7.4 |
| `String(i)` | 21,171 | 1,694 | 12.5 |
| `s.padEnd(20)` | 40,232 | 2,944 | 13.7 |
| `Math.max(i, 3)` | 25,723 | 1,484 | 17.3 |

So in the shipped configuration, **plain bytecode is already at parity with
quickjs-wasi** (0.7-1.3x per op), and the order of magnitude is entirely in builtins and
iteration (4-17x per call). Caveat on the parity: V8 has no on-stack replacement for
wasm, and quickjs-wasi's `JS_CallInternal` runs a whole top-level loop in one
activation, so its loops stay in Liftoff code (as they do in pi today). With
`--no-liftoff` for both (everything TurboFan, `wmeasure-noliftoff.txt`) the plain
workloads are 1.45-2.1x qjs-wasi's instructions (tight 1.53, loop 1.72, call1 1.69,
calls 2.08, prop1 1.45) and the builtin ones 2.9-8.3x (pi-like 2.85, forof 8.3,
indexof 8.3, pushpop 4.0). The same no-OSR rule bites djs: a single huge `vm_run` slice
keeps `run_loop` in Liftoff for the whole run: tight is 25.5G instructions with one
slice vs 13.9G with 10k or 1M slices (1.8x). The Sandbox (1M) and pi (10k) defaults are
fine; an embedder must never run one giant slice.

Native, for reference (djs vs qjs native, `measure.py`, 6c7823a): tight 4.6x
instructions (3.3x cycles), call1 3.3x, prop1 4.0x, pushpop 5.4x, pi-like 4.0x
(3.9x cycles), forof 13.6x (18.6x cycles), indexof 16.7x, strbuild 3.1x, json 0.53x,
jsonh 0.70x. qjs native has computed-goto dispatch and refcounting; djs native runs
~50 machine instructions per simple bytecode, djs wasm (TurboFan) ~80, quickjs-wasi's
Liftoff loop ~90.

### F1. Where pi-like's work goes: 95% of executed bytecode is the builtin language

Instrumented counts (pi-like, 150 reps): 67.4M bytecodes, **64.1M (95%) in prelude
builtins**, 4.9M bytecode frames, 4.55M `call_intrinsic` from bytecode (68% on the slow
path), 1.8M cells / 169 MB allocated, 299 collections. By function:

| function | bytecodes | calls | per call |
| --- | ---: | ---: | ---: |
| StringPrototypeSplit (bytecode) | 15.5M | 9,000 | 1,724 |
| StringPrototypeIncludes | 14.7M | 445,500 | 33 |
| ThisToString, ClampPosition | 6.7M + 6.7M | 448,500 + 445,500 | 15 + 15 |
| ToString, ToIntegerOrInfinity, ToNumber, IsRegExp, StrLen | 5.5 + 2.9 + 2.9 + 2.7 + 1.4M | 0.5-0.9M each | 3-6 |

`line.includes("throw")` is 7 bytecode frames and ~90 bytecodes; split and includes
plus their helpers are **~59M of the 67M bytecodes and ~4.4M of the 4.9M frames**. In
wasm the profile of pi-like is `run_loop` 30% self, `call_intrinsic` 4%,
`string::index_of` 3%, GC (`for_each_slot`, `alloc_init`) 6%, `load_frame` 2.4%,
`n_slot` 2.1%. qjs runs both builtins as C. Everyday builtins written in the builtin
language and decomposed into small helper functions (spec style) are the single
largest cause of the codemode gap: a helper call costs ~420 native instructions
(F4), more than the work it wraps.

### F2. Per bytecode: ~50 native instructions for `get_loc`, of which ~5 are the work

Exact trace of one `get_loc` in the native build (52 instructions, 15 loads, 4 stores,
10 branches), condensed:

```
loop top:   cmp x8,#1 ; b.lt                         fuel (x8 forwarded from the last store)
fast_step:  ldr w10,[x20,#0x28]  ldr w9,[x20,#0x34]  cmp ; b.hs     pc, code_len, pc < code_len
            ldr x24,[x20,#8]     ldr w12,[x20,#0x30]               &Heap, code offset
            ldr x1,[x24,#0x10]   add ; cmp ; b.ls                  arena Vec len, bounds check
            ldr x0,[x24,#8]      ldrb w21,[x0,x11]                 arena Vec ptr, opcode
            cmp w21,#0x99 ; b.hi ; ldr w9,[x20,#0x24]               range check, sp
            adrp ; add ; adr ; ldrh ; add ; br                     jump table (u16 relative)
get_loc:    add ; subs ; ccmp ; b.ls ; ldrh                        imm u16: bounds check + load
            ldr w10,[x20,#0x3c] ; add                              lp
            ldur w10,[x20,#0x18] ; add #16 ; add lsl#3              stack cell, Tail::at
            subs ; ccmp ; b.hi                                     bounds check (local)
            add lsl#3 ; subs ; b.lo ; cmp #7 ; b.ls                bounds check (push slot)
            ldr x8,[x0,x8] ; str x8,[x0,x10]                       THE WORK: load local, push
            add ; str w8,[x20,#0x24]                               sp += 1 (memory)
            ldr x8,[x20,#0x10] ; sub ; str                         fuel -= 1 (memory RMW)
            ldr w9,[x20,#0x28] ; add #3 ; str                      pc += 3 (memory RMW)
            b loop
```

Per op (trace_report, native): `get_loc` 52, `put_loc` 54, `drop` 31, `jump` 41,
`jump_if_false` 51, `lt`/`add` 54 (int), `push_i32` 46, `get_field` (IC hit) 132,
`put_field` 131, `get_global` 218, `arr.length` 419 (no IC: `length` is not a table
entry), slow-path ops `insert2` 125, `swap` 112, `dup2` 118, `iter_check_object` 89,
`special_object` 148, `bor` 186 (not in `fast_step`). Across traces the instructions
split into: **bounds checks 19-26%**, dispatch (pc/code_len/opcode/table) 17-37%, Exec
state loads/stores 10-13%, address arithmetic 4-9%, NaN-box tag tests 3-8%, the actual
loads/stores of values ~5%.

The wasm arm is the same shape (wasm_arm.py on 32b9f05): `get_loc` is 71 wasm
instructions with 5 bounds-check `br_if`s (each two-sided: `le_u`, then `sub; le_u`),
the state as `i32.load/store` on `self` in linear memory (`[self+68]` pc, `[self+64]` sp,
`[self+8]` fuel as i64, `[self+88]` lp, `[self+52]` stack, `[self+48]` → heap → Vec
ptr/len), plus a ~30-instruction head where LLVM lowered the opcode switch as a
compare tree (`> 110`, `> 69`) in front of five `br_table`s. TurboFan turns this into
~80 machine instructions per simple op (tight: 14.6G / 180M ops).

Root causes, in order of weight:
1. **State in memory.** `pc`, `sp`, `fuel`, `lp`, `code`, `code_len`, `stack` are `Exec`
   fields; the loop body contains non-inlined calls taking `&mut self` (`step`,
   `call_intrinsic`, ...), so LLVM keeps them in memory and does a load-modify-store per
   op (pc and fuel), and reloads after every store. The arena is reached through
   `self.heap` (a pointer loaded from `Exec`) → `Vec` ptr and len: a dependent chain of
   three loads before any value access, and since a store into the arena may alias the
   Vec header (not provably distinct), **ptr and len are reloaded after every arena
   store** (the six header writes of `push_frame` each reload both: 9-10 instructions
   per 8-byte store).
2. **Two-sided bounds checks on every access.** `bytes.get(off..).first_chunk::<N>()`
   checks `off <= len` and `len - off >= N` (3-4 instructions); a typical op does 3-4,
   a call ~20. Measured by removing them (scratch copy, unchecked reads/writes): wasm
   **-11 to -24% instructions** (tight -17, call1 -23, prop1 -20, forof -22, pushpop
   -22, indexof -24, pi-like -18, higher-order -23, strbuild -11, jsonh -12) and -5 to
   -27% cycles; native -21 to -32% instructions; wasm code -39 KB. In wasm every load
   is already confined to linear memory, so these are second checks (D016).
3. **Dispatch.** `pc < code_len` check (it exists to route native frames to
   `native_step`), opcode range check, a 154-entry relative table; in wasm a compare
   tree plus `br_table`s because the fast opcodes are scattered over 0..153.
4. **Fuel** as a memory RMW per op plus a compare at the top (5 instructions).

The model (scratch `model/`, the same 17-op loop over a `Vec<u8>` arena with u32
offsets, NaN-boxed values): djs style (state in a struct, two-sided checks,
out-of-line `&mut self` slow path) 37.6 instr/op and 4.9 cycles/op; **state in locals
and a local `&mut [u8]` re-derived after the slow path, one-sided checks (safe Rust):
26.9 instr/op, 4.1 cycles/op (-29%)**; plus unchecked accesses: 21.2 instr/op, 3.3
cycles/op (-44%). The real loop sits ~12 instructions above the model's djs style (frame
features, the much larger function, `code_len` routing).

### F3. Native (bgen) state machines: ~500 instructions per array element

Per array element (`perelem.py`, native, instructions):

| per element | djs | qjs | ratio |
| --- | ---: | ---: | ---: |
| `for (const x of a) t += x` | 5,433 | 386 | 14.1 |
| `for (let i…) t += a[i]` | 1,549 | 308 | 5.0 |
| `a.forEach` / `a.map` / `a.filter` / `a.reduce` | 1,758 / 1,778 / 1,927 / 1,495 | 531 / 735 / 772 / 510 | 2.4-3.3 |
| `a.indexOf(-1)` / `a.includes(-1)` | 490 / 653 | 28 / 34 | 17-20 |
| `a.join(',')` | 2,424 | 469 | 5.2 |
| `a.slice()` | 945 | 20 | **48** |
| `[...a]` (Rust spread, D1351) | 459 | 292 | 1.6 |
| `a.sort((x, y) => x - y)` (sorted) | 14,377 | 2,650 | 5.4 |
| `s.split('\n')` per piece (bytecode builtin) | 4,728 | 555 | 8.5 |

`indexOf`'s loop is 6 state arms per element (14 → 15 HasProperty → 18 → 19 Get → 20 →
16 back-edge), each arm a `match ex.n_state()` through `pc` in memory. Every local is a
Value slot in the arena stack (checked load/store, NaN-box encode/decode via
`num(f64_of(..))`), inline fast paths push their result on the operand stack and the
next arm pops it (`n_inline`/`n_result`), every back-edge charges fuel and checks the
safepoint, and leaf helpers go through the generic `call_intrinsic` (a 32 KB wasm
function; args pushed on the stack). In wasm `n_slot`, `n_set_slot`, `n_this`, `n_arg`,
`f64_of`, `num` are out of line (D1634), so each slot access is a call: the wasm profile
of indexof is `n_slot` 15.7%, `native_ArrayPrototypeIndexOf` 14.0%,
`get_element_fast` 8.6%, `n_has_property` 6.7%, `n_get` 6.6%, `n_set_slot` 5.5%,
`f64_of` 4.4%, `n_result` 3.8%. Two smaller finds:
- `iid("Set")` / `iid("Get")` / `iid("HasProperty")` / ... in `native.rs`'s slow paths
  are `const fn` calls in a **non-const context**, so they run: a linear string search
  over the intrinsic table at every call (`id` is 103 instructions per push/pop native
  segment natively, 2.2% of pushpop's wasm time).
- `n_str("length")` maps literal names to atoms by string compares at runtime.

### F4. Calls and returns: ~420 native instructions per pair

Fast-path `call` 234 instructions (48 loads, 15 stores, 72 of them bounds checks),
`return` 185 (42 loads, 30 tag tests decoding header ints): `fast_call_at` reads the
callee's class, slot 0, template `PARAMS`, `LOCALS`, `STACK`, `FLAGS` and the stack
capacity, each a separate checked load; `push_frame` writes six NaN-boxed header Values
(each: encode with `mov/movk/orr`, Vec ptr/len reload, two-sided check, store) and
initializes locals in a loop; `enter_template` follows `template → CODE → Bytes::LEN`
and `CONSTS`; the return goes back through `load_frame` (`stack[bp]` → callee → slot 0
→ template → code → length) and a safepoint check. Calls into native builtins take the
slow `call` path (~470-530 instructions: `push_native_frame` with `ensure`, header,
locals). In wasm `call+return f(i)` is 1,211 machine instructions vs 966 for
quickjs-wasi.

### F5. for-of: a native frame, six generic intrinsic calls and two cells per element

Per element: `dup2; swap; call 0` (slow path) → `push_native_frame` for
`%ArrayIteratorPrototype%.next` → 9 state arms with 6 `n_leaf` → `call_intrinsic`
(ArrayIteratorTarget, Index, IsTypedArray, SetIndex, Kind, CreateIterResultObject) →
`{value, done}` (an Object cell plus a PropTable cell) → `n_ret` → `iter_check_object`,
`get_field done`, `get_field value`. 5,433 native instructions per element; in wasm the
profile is `run_loop` 18%, `call_intrinsic` 10%, `n_leaf` 9%, GC `for_each_slot` 6%,
`call` 5.4%, `n_slot` 5.2%, `table_insert` 2.7%, `new_object_from_entries` 2.2%. qjs
reads the next element of a fast array directly. D1292 noted the missing fast path.

### F6. Allocation, GC and memory

- The iterator result costs 144 B in two cells at 6c7823a (24 B object + 120 B table;
  0083's 16-byte entries shrink the table); 4M of them in forof → 230 collections, each
  copying the live 1.6 MB `xs` array again (Cheney, threshold 2x live): forof spends
  **20% in GC** natively, plus 8.4% `memset` zeroing the arena frontier for new tables
  (`zero_ahead`), 10% `memcpy` in GC forwarding, 2.5% `realloc` (each collection
  allocates a fresh to-space `Arena::with_capacity(before)`). pi-like: 12.6% GC.
- In wasm32 linear memory never shrinks: peak memory is from-space + to-space + Vec
  doubling slack. Wasm memory after a run: strbuild 57.9 MiB, higher-order 49.3,
  arrays 30.4, forof 22.8, json 21.1 (qjs native peak RSS for the same: 15.4, 14.7,
  19.2, 6.4, 4.4 MB).
- Transient Rust heap allocations in leaf paths: a short `'a' + i` runs ~2,200
  instructions in `libsystem_malloc` (`concat` → `collect_leaves` allocates three Vecs
  for two flat strings; number-to-string allocates), ~3,300 in total for the `add`.
  In wasm that is dlmalloc, out of the same linear memory.

### F7. The compiler emits ~1.5x the bytecodes

`for (let i = 0; i < n; i++) s += i` runs 18 bytecodes per iteration: `i++` as a
statement is `get_loc; to_numeric; dup; inc; put_loc; drop` (qjs's peephole makes it
`inc_loc`), `s += i` is `get_loc; get_loc; add; set_loc; drop`, the loop takes three
jumps per iteration (`jump_if_false` and two `jump`s), and an assignment statement
`o.c = i` is `get_loc; get_loc; insert2; put_field; drop` where `insert2` is a 125-
instruction slow op. Prelude code also runs common simple ops on the slow path:
pi-like's `get_cap` (3.8M, 100% slow), `strict_eq` (1.5M), `not`, `null`, `push_false`,
`special_object`, `push_const` cost 90-190 instructions each instead of ~45.

## Proposals

Ranked by expected gain in wasm on codemode-like work (pi-like; F0's builtin rows).
"Gain" is instructions unless noted; the estimates come from the counts above.

**Cheap (days each, no architecture change):**

1. **Leaf fast paths for the everyday String/Math/Number/Object builtins.** When the
   receiver and arguments are primitives (or a plain object for `hasOwnProperty`),
   nothing can run user code, so `includes`, `indexOf`, `startsWith`/`endsWith`,
   `slice`/`substring`, `split` with a string separator, `trim*`, `padStart`/`padEnd`,
   `String(x)` of a primitive, `Math.max`/`min`/`floor`, `hasOwnProperty`, ... become one
   Rust intrinsic each, called first from the builtin (the general path stays the TS
   source). This is the leaf rule as designed; JSON shows the result (0.6-0.75x
   qjs-wasi). Gain: 5-17x per call (F0 table); **pi-like about -50%** (59M of 67M
   bytecodes, 4.4M of 4.9M frames are split/includes and their helpers). Risk: none to
   the guarantees; both backends stay testable. Code size: small, table-driven where the
   arms share argument checks.
2. **for-of fast path** (D1351/D1346 identity checks): while the iterator is an Array
   Iterator with the installed `next`, a `for_of_next`-style op reads the next dense
   element directly, no call, no result object; otherwise today's protocol. Make
   `ArrayIteratorPrototypeNext` itself a leaf intrinsic for arrays (no native frame).
   Gain: forof 6x → ~1x qjs-wasi; jsonh, pi-like loops. Risk: low, precedent exists.
3. **bgen/native runtime quick fixes**: `const { iid(..) }` for the runtime intrinsic
   ids (F3); atoms for `n_str` literals resolved at generation time; dense-Array leaf
   loops for `indexOf`/`includes`/`slice`/`join`/`lastIndexOf`/`fill` (a dense array
   without holes cannot run user code, like spread's path): indexOf/includes 17-20x →
   ~1-2x, slice 48x → ~1x.
4. **Simple ops into the fast path, and compiler peepholes** (F7): `get_cap`,
   `strict_eq`/`strict_ne` on primitives, `not`, `null`, `push_true/false`, `swap`,
   `dup2`, `nip`, `insert2`, int bitops, `push_const`, `iter_check_object`; and
   `inc_loc`/`dec_loc` for `i++` statements, compound assignment and assignment
   statements without `set+drop`/`insert2+drop`, loop condition at the bottom (one jump
   per iteration), `arr.length` in the IC/fast path. Gain: plain loops -30-40%
   bytecodes, prelude -5-10%. Renumber the fast opcodes into one dense range so wasm gets
   a single `br_table` (~6 wasm instructions per op). Mind D1350/D1352's
   register-allocation fragility: measure each with interleaved A/B, which item 6 makes
   less fragile.
5. **Allocation quick wins**: reuse the GC to-space buffer across collections (no
   realloc, no page faults; in wasm no extra `memory.grow`), no transient `Vec`s in
   `concat` of two flat strings and in number formatting (a stack buffer), and a GC
   threshold policy that accounts for 64 KiB pages and never doubles past need.

**Structural (weeks, each needs a decision):**

6. **Run-loop state in locals** (F2): `pc`, `sp`, `fuel`, `lp`, the code base and the
   stack base live in locals, and the arena as a local slice, re-derived after anything
   that can allocate or collect (the existing safepoint discipline: the frame cache is
   already re-derived after every safepoint). Sync to `Exec` only around slow paths,
   calls and safepoints; one-sided checks; fuel as `subs; b.le` in a register; split the
   cold arms out of the 49 KB `run_loop` (TurboFan compiles it only between slices, and
   compiles a smaller function better). Gain: model -29% per op in safe Rust; real loop
   ~50 → ~30 native, ~80 → ~45-50 wasm machine instructions per simple op; plain
   bytecode -25-40%. Risk: GC-moved offsets if a re-derivation is missed (the gc-stress
   mode catches it); no change to images or suspension.
7. **Bounds checks once per cell or frame, not per access** (D016, 0052 S3): the frame is
   checked once at entry (`frame_top`), so slots within a checked frame and fields of a
   checked cell can use a slice of that region; header writes as one checked block.
   Measured ceiling for removing all checks: wasm -11..-24% instructions. A fully
   unchecked wasm build is a separate decision: wasm confines it to the instance's
   linear memory, but a corrupted image could then corrupt the Rust heap inside it.
8. **bgen backend: Rust locals between resume points.** Values that do not live across a
   resume point stay Rust locals (bgen already knows the points); slots only at resume
   points; fast paths return values instead of push/pop; leaf intrinsics as direct Rust
   calls instead of `call_intrinsic`; one state transition per loop iteration. Gain:
   F3's per-element costs 3-10x down (a native loop becomes a Rust loop with a fuel
   check), higher-order 3.9x → ~1.5x; in wasm most of D1634's out-of-line helper calls
   disappear, and code shrinks. Risk: the backends' equivalence (tested, D1635).
9. **A compact call protocol** (F4): the template's call data in one packed word
   (params, locals, stack, flags) plus code offset and length cached in the template;
   the caller's template as an internal ref in the header, so a return is two loads and
   no `load_frame` chain; header written as one checked block; no locals loop for
   functions without locals. Gain: call+return ~420 → ~150-200 native instructions;
   calls -40%. Risk: frame layout (images, `Vm::backtrace`) changes; GC already traces
   header Values.
10. **Memory**: shapes (0083, in progress) for `{value, done}`-like objects; a
    large-object space for big arrays and strings (ARCH plans it) so a collection does
    not copy a 1.6 MB array 230 times.

**Ceiling for this design** (stack VM, NaN-boxed values, arena offsets, frames in the
arena, fuel, no JIT): in wasm, plain bytecode is already at quickjs-wasi parity as pi
runs it; with 4, 6 and 7 it should be ~0.5-0.7x qjs-wasi's instructions (about parity
with qjs-wasi fully tiered). Builtin-heavy code can reach parity or better with 1-3 and
8, as JSON shows. Codemode overall: pi-like 3.1x cycles today → ~1.2-1.5x with 1-3,
~1x or below with 6-9. Natively the floor is higher: qjs's computed-goto C loop runs
~16 instructions per op, djs could reach ~25-30 in safe Rust (model: 27), so plain
loops stay ~1.5-2x qjs native unless bounds checks go. What stays inherent: the
offset-plus-base addressing (one add, free on arm64, explicit in wasm), NaN-box tag tests
(qjs has them too), fuel (2 instructions per op), one bounds check per cell/frame if
kept, and no OSR in V8's wasm tier.

## Follow-ups

Candidate issues, in the order of the Proposals (each with a wasm before/after from
`scripts/perf/wmeasure.py` and `wmicro.py`):
- leaf fast paths for the everyday String/Math/Number/Object builtins (proposal 1);
- the for-of / Array Iterator fast path (2);
- native runtime quick fixes: `const` intrinsic ids, literal atoms, dense-array leaf
  loops (3);
- fast-path coverage and compiler peepholes, dense fast-opcode numbering for wasm (4);
- GC to-space reuse, no transient Vecs in concat/number formatting, page-aware threshold (5);
- run-loop state in locals with a split `run_loop` (6, needs a decision);
- bounds checks per cell/frame (7, with 0052 S3; an unchecked wasm build is a separate
  decision);
- bgen: Rust locals between resume points (8);
- compact call protocol (9);
- large-object space; shapes are 0083 (10).

## Result
Investigation only, no engine changes. The issue's Findings (F0-F7) and Proposals
are the report; scripts/perf/ holds the tools (README there): native and wasm
instruction-count harnesses against qjs and quickjs-wasi, per-operation and
per-element micro measurements, an lldb-based exact per-bytecode instruction tracer,
an instrumented engine copy, samply/cpu-prof analysis, a wasm arm reader, the
no-bounds-check experiment and a dispatch-cost model.

Main results: in wasm with pi's fuel slicing, plain bytecode runs at quickjs-wasi parity
(0.7-1.3x per op), and the gap is in builtins and iteration: 4-17x per call of
everyday String/Math/Array builtins, for-of 6x, pi-like 3.6x instructions and 3.1x
cycles, 95% of whose bytecode is builtin-language code (split/includes and their
helpers alone are 88%). Natively, a simple bytecode costs ~50 instructions (wasm ~80),
of which ~5 are the work. The rest is state kept in memory, two-sided arena bounds
checks (removing them: -11 to -24% wasm instructions), dispatch and fuel. A call+return
costs ~420 native instructions. bgen's native loops cost ~500 instructions per array
element. Known gaps: no V8 machine code (node's release build has no disassembler), so
the wasm side is the wasm code plus instruction counts; quickjs-wasi's top-level loops
run in Liftoff (no OSR), as they do in pi.
