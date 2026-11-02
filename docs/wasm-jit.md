# A JIT that emits wasm: performance and fit

Issue 0153, research only: no engine change on main. The owner's question: "One potentially
quite nice aspect of how we do the builtins is that it should be possible to build a JIT
that emits more wasm. Investigate this, both in terms of performance and whether it makes
sense for our setup."

Everything below was measured on an arm64 Mac (node 25.8.1, V8 14.x), with the `wasm`
profile build of djs-wasm at 440e23ed. The prototype is in `work/scratch/0153/proto` of
the jit worktree, and it is kept on the branch `scratch/0153-wasm-jit-proto` (not merged).
Wall-clock numbers vary by ±10–15% between runs on this shared machine. The ratios are
stable.

## In short

- **A JIT that emits wasm can be built without breaking djs's model.** The prototype is a
  template compiler from djs bytecode to wasm. It meets three conditions:
  - compiled code exits only at instruction boundaries;
  - it checks fuel once per basic block, so it stops exactly where the interpreter
    would;
  - it writes its registers back at every exit.

  Under those conditions, suspend-anywhere, fuel, determinism, the moving collector and
  snapshots all hold. They held in all 60 oracle runs: 5 programs, 4 fuel-slice sizes,
  3 compiled variants. Results, slice counts and the live state after every slice were
  identical to the interpreter's.
- **The speed-up is real once V8 has optimized the code.** With TurboFan, the
  template-compiled code is **3.2–4.5x faster than djs's interpreter** on loops and calls
  (and 7.9x on a property loop with a simplified cache). The cost of exact fuel within
  that is about 10%. With V8's baseline compiler (Liftoff) only, the same code runs
  **0.7–1.8x the interpreter's speed** (3x on the property loop).
- **That makes the JIT useless for codemode.** The 300 real pi scripts in the harvest
  corpus run for a median of **0.12 ms** in djs-wasm (p99 1.4 ms, all 300 together
  55 ms). V8 needs a few hundred µs of compile per function, and a function only tiers up
  to TurboFan after millions of executed instructions. In the prototype, a hot function
  pays off after **~30,000–50,000 loop iterations**, about 0.3 ms of interpreted time in
  that one function. No harvested script gets there.
- **The heavy workloads are not dispatch-bound either.** On pi-like, the interpreter's
  dispatch and fast paths are 22% of wasm time. JSON, strings, allocation and GC are the
  rest, and a JIT does not touch them. The JIT's ceiling there is about 10%.
- **The builtins already have their "emit more wasm" path:** `@native`, the bgen Rust
  backend, compiled by LLVM into the module. It gives pi-like 3.3x over the bytecode
  backend. It costs module size (24 KB of code for 10 builtins), so each builtin has to
  justify it with a measurement (D1634). A bgen wasm backend for all builtins in a
  separate, lazily loaded module would avoid the size cost. But it needs a cross-module
  ABI of about 300 intrinsic exports, and its payoff is bounded by the time spent in
  bytecode builtins that are not `@native`. That time is a part of the 12% `run_hot`
  share on pi-like.
- **Recommendation: don't build a JIT now** (D2510).
  - Keep `@native` as the AOT path, justified per builtin.
  - Keep the interpreter work (0151's denser bytecode, fused instructions).
  - If a non-codemode use ever needs compute speed, the variant to build first is the
    prototype's: a baseline template JIT, host-driven like pico's, off by default, with
    the interpreter as its exact oracle.

## 1. Where djs spends its time

### Profiles in wasm

CPU profiles of `scripts/perf/wasm-run.ts` (node `--cpu-prof`, the `wasm` build with
names) give each function's self time, bucketed by
`work/scratch/0153/prof/buckets.py`. Shares are of wasm time only:

| workload | run_hot (dispatch + fast paths) | general path | ICs, fast calls, `%EachDense` | @native state machines | leaf builtins and heap ops | JSON | strings, regexp | GC | alloc |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| pi-like | 11.8% | 2.4% | 8.1% | 2.5% | 18.3% | 21.0% | 8.9% | 11.5% | 10.8% |
| higher-order | 23.8% | 1.3% | 20.2% | 33.8% | 11.7% | – | – | 6.8% | 0.5% |
| props | 51.7% | 9.5% | 2.7% | – | 19.6% | – | 7.2% | 3.8% | 2.7% |
| arrays | 64.2% | 0.1% | 4.5% | – | 25.6% | – | – | 3.4% | 0.4% |
| calls | 94.4% | 0.3% | – | – | 0.2% | – | – | – | 0.5% |
| strings | 15.2% | 2.7% | 3.1% | – | 14.1% | – | 30.2% | 4.2% | 21.1% |
| json | 2.4% | 0.8% | – | – | 13.0% | 43.9% | 0.8% | 16.3% | 7.3% |

A JIT replaces the `run_hot` column: decoding, dispatch, and keeping the operand stack
in memory. Its inline caches, fast calls and leaf builtins would still run, called from
compiled code. So the ceiling is high only on the synthetic interpreter-bound workloads
(calls, arrays, props, the loops). pi-like, the closest to codemode, has 12% in
`run_hot`. Even at 4x speed on that part, it saves about 9%.

### Real codemode scripts

`work/scratch/0153/harvest/run.ts` runs each of the 300 harvested pi scripts
(`tests/corpus/codemode-harvest`, tool results synthetic) in djs-wasm. The time is
compile plus run. Fuel is counted to within the 1000-unit slice:

| | run time | fuel |
| --- | ---: | ---: |
| median | 0.12 ms | ~3,000 |
| p90 | 0.30 ms | ~7,000 |
| p99 | 1.4 ms | ~95,000 |
| max | 1.8 ms | ~105,000 |
| all 300 | 55 ms | 1.57 M |

A whole median script executes about as many instructions as 330 iterations of a small
loop. The real scripts spend their time waiting for tools, which no engine speed changes.

### The AOT path that exists: `@native`

The same workloads, natively (`djs run --body`, wall time), with the `@native` builtins
as Rust state machines (the default) and as bytecode (`--backend bytecode`):

| workload | native backend | bytecode backend |
| --- | ---: | ---: |
| pi-like | 0.09 s | 0.30 s |
| higher-order | 0.12 s | 0.26 s |
| foreach | 0.09 s | 0.18 s |
| arrays | 0.18 s | 0.63 s |
| strings | 0.03 s | 0.10 s |
| props, calls, json | unchanged | unchanged |

Ten builtins are `@native`. Their generated code is 24.3 KB in 13 wasm functions, plus
10 KB of native runtime (`interp/run/native.rs`), against 16 KB for `run_hot`. That is
the "builtins compile to more wasm" idea, already in place. LLVM compiles the code ahead
of time, and V8 compiles it once per process with the module.

## 2. The options

### a. AOT: builtins compiled to wasm at build time

1. **More `@native`** (bgen's Rust backend). This gives the best code (LLVM, then
   TurboFan) and no compile work in a VM. Its only cost is module size: about 2.4 KB of
   code per builtin on average, against the gzip budget of D020. Every builtin moved
   must win in a benchmark (D1634). That rule stays right: most builtins are cold in
   codemode.
2. **A bgen wasm backend in a side module.** bgen would emit wasm functions directly
   from the typed subset into `builtins.wasm`. Numbers would be unboxed `i32`/`f64`
   where the subset says so, and values boxed. Resume points would be `br_table` states,
   like the prototype's blocks and `@native`'s arms. The module would import djs-wasm's
   memory, function table and intrinsics. The host would load it lazily, and V8 would
   compile it once per process. Pros:
   - no main-module size;
   - typed code, better than a JS template JIT;
   - no runtime emitter in djs-core;
   - testable against the bytecode backend exactly as `@native` is (D1635).

   Cons:
   - a third bgen backend;
   - djs-wasm must export the ~300 intrinsics and the native-frame API. Exports cannot
     be inlined or stripped, and every intrinsic call becomes a cross-module call;
   - the payoff is bounded by the time spent in builtins that are bytecode today. The
     hot ones are already `@native`.
3. **The Rust backend compiled into a second Rust module.** This needs wasm dynamic
   linking: shared memory, position-independent code, `wasm-ld -shared`. That is not a
   practical toolchain for a `no_std` crate today.

### b. A runtime JIT of user code (and of bytecode builtins)

1. djs-core emits wasm bytes for a hot template. This is plain byte writing, so it is
   `no_std` and safe.
2. djs-wasm exposes the bytes through exports, like pico's `pico_jit_take` and
   `pico_jit_install`.
3. The host compiles them (`WebAssembly.compile`, asynchronously, between fuel slices)
   and instantiates them against djs-wasm's memory and table. It appends the export to
   the table and reports the index back.
4. The interpreter enters compiled code with `call_indirect` at block leaders: function
   entry, loop headers and return points.

pico does exactly this (its "The JIT" chapter, summarized in
docs/lessons-from-pico.md). The prototype below does it for djs bytecode.

### c. Baseline template versus an optimizing tier

- **Baseline** (the prototype). Each instruction becomes a fixed wasm sequence on boxed
  values with the interpreter's own fast paths (int32 and float arithmetic, compare and
  branch fused, inline-cache hits, calls and returns of closures), and anything else
  exits. There is no type feedback and no speculation, and deoptimization is just an
  exit.
- **Optimizing.** This tier would speculate from the inline caches: int32 locals
  unboxed, shape guards, inlined property access, guarded call targets and inlining. A
  failed guard deoptimizes, which costs little in djs: the exit at an instruction
  boundary is already there, and the state just has to be materialized. Against that:
  - the interpreter must collect type feedback, which slows it and adds per-VM state
    that must be deterministic and image-safe;
  - TurboFan compiles grow;
  - it is a V8-sized project.

  JavaScript has no sound types, so pico's "typed without speculation" shortcut is not
  available. For codemode, where even the baseline cannot pay off, this tier is out of
  the question.

## 3. Fit with djs's invariants

- **Suspend anywhere (G5).** Compiled activations exist only on the wasm stack, during
  one entry from the interpreter. At every exit (fuel, a case off the fast path, a
  safepoint), compiled code writes its locals and live operand-stack slots into the
  frame and saves `pc`/`sp`/fuel. The result is the state the interpreter would have
  reached. With nested compiled calls, each activation flushes, innermost first, and
  returns. The interpreter then runs the callee and re-enters the caller's compiled code
  at the return point. The prototype's oracle checks exactly this.
- **Fuel and determinism.** At block entry, compiled code checks `fuel > (cost of every
  instruction but the last)`, then subtracts the block's cost. If the check fails, it
  exits at the block start, and the interpreter single-steps to the exact boundary where
  it would have stopped (it may not re-enter at the pc it just left). A side exit in the
  middle of a block refunds the rest of the block. The fuel totals are identical, and
  so is the slice in which the run ends. Further rules:
  - Non-canonical NaNs must never reach memory. The prototype canonicalizes as
    `conv::number` does.
  - Which code runs, compiled or interpreted, depends on when the host finished
    compiling, so it must not be observable. That holds only while the identity
    invariant holds.
  - Heat counters must be fuel-based or kept out of images.
- **Dead stack slots.** The interpreter leaves popped values above `sp`, and images
  include the whole value stack (`compact.rs` copies it at `IMAGE_STACK`). Compiled code
  keeps operands in wasm locals, so the slots above `sp` differ. Behaviour is the same,
  and `safepoint_collect` already clears those slots before collecting. But images would
  only be byte-identical, and the oracle that strong, if suspension (or the image
  writer) cleared them too. The prototype compares live slots only.
- **The moving collector.** Collections happen only at safepoints. Compiled code exits
  there when a collection is pending (`gc_pending`), so no compiled frame ever sees a
  collection. Two consequences:
  - compiled code must not embed arena offsets (templates, inline-cache arrays,
    constant pools all move); it loads them at entry from the frame;
  - the arena is a `Vec<u8>` that can be reallocated when it grows, so its base address
    must be reloaded after any helper that allocates (or the arena placed at a fixed
    address).

  There is no write barrier today (docs/old-generation.md, phase 1). A card barrier
  (phase 2) would have to be replicated in emitted code.
- **Snapshots.** Code is never in an image. The template's compiled-code slot (a table
  index) must live outside the arena, or carry a per-VM epoch, so that a restored VM
  starts uncompiled. Modules can be cached per process by their bytes, as pico does.
- **`no_std` and no `unsafe` in djs-core.** The emitter and its analysis (blocks, stack
  depths, resume points) are safe byte writing. Calling a table index needs a function
  pointer made from an integer, which is `unsafe` and belongs in djs-wasm. Helpers
  called *from* compiled code (an inline-cache miss, the general path of an instruction)
  re-enter Rust while the run loop holds `&mut` borrows. There are two ways out:
  - only exits, no helpers (the prototype), which costs an exit and a re-entry per slow
    operation;
  - helper exports reached through a raw pointer, with djs-core releasing its borrows
    around the call, as `heap_call!` does for the arena slice. That is more `unsafe` in
    djs-wasm.
- **Memory safety inside the instance.** djs-core checks every arena access. Emitted
  code addresses linear memory directly, so a JIT bug can write into Rust's allocator
  heap outside the arena. The wasm sandbox still protects the host: the blast radius is
  the one VM in its instance. Explicit bounds checks against the arena length cost a
  load and a compare per access.
- **The ABI.** djs-wasm imports nothing. Its function table is internal and fixed
  (`initial=max=136`). A JIT needs:
  - the table exported and growable (`--export-table --growable-table`);
  - about 3–10 JIT exports (take, install, a cache hint, stats);
  - a driver in js/src/sandbox.ts that compiles between slices and caches modules.

  The host does the compiling, as with pico. djs-wasm can stay import-free.
- **V8 specifics.**
  - Wasm has no on-stack replacement: a long loop stays in Liftoff until its function is
    entered again. Fuel slices provide those re-entries, as `HOT_BACKEDGES` does for
    `run_hot` today.
  - Dynamic tiering only optimizes a function after a budget of about 13 M "bytes
    executed" (`--wasm-tiering-budget`).
  - V8 inlines `call_indirect` targets speculatively, and a new instance's targets
    differ, which deoptimized pico's interpreter loop. djs should call compiled code
    through a small trampoline, not from `run_loop`.
  - One 1.4 MB function (3,000 blocks in one dispatch loop) made TurboFan abort with a
    zone OOM (section 4). Functions must be capped or split.
- **Other hosts.** Browsers compile wasm at run time like node. A CSP needs
  `'wasm-unsafe-eval'`, which djs needs for its own module anyway. Wasmtime and other
  native embeddings can compile modules at run time (Cranelift, with no lazy tiering),
  but only if the embedder implements the host side. pi runs on node.

## 4. The prototype

`work/scratch/0153/proto` (branch `scratch/0153-wasm-jit-proto`). It is written in
TypeScript and run from node:

- `bytecode.ts` reads djs's opcode table (sizes, stack effects, fuel classes) from
  docs/guide/ref-opcodes.md. It runs `djs disasm` on progs.js and parses real djs
  bytecode from main.
- `engine.ts` is a miniature of djs's execution model in one `WebAssembly.Memory`:
  - djs's NaN-boxed values;
  - the value stack with djs's frame layout (callee, `this`, padded arguments, the
    6-word header, locals, operand stack);
  - objects with a shape word and slots, closures whose slot 0 is their template, and
    capture boxes;
  - per-template inline caches;
  - the `Interp` registers.

  Its interpreter is the reference semantics for the subset, with djs's fuel costs and
  djs's stopping rule. With a JIT attached, it enters compiled code at block leaders.
- `jit.ts` is the template compiler for 26 opcodes, with one module per template that
  imports the memory and the table:
  - `push_i8`/`push_i32`/`undefined`;
  - `get_loc`/`put_loc`/`set_loc`/`get_arg`;
  - `inc_loc`/`dec_loc`;
  - `add`/`sub`/`mul`/`sar`/`band`, with int32 fast paths, overflow checks and float
    helpers that renormalize like `conv::number`;
  - `lt`/`gt`, fused with a following `jump_if_*`;
  - `jump`/`jump_if_true`/`jump_if_false`;
  - `get_cap`;
  - `get_field`/`put_field` (an own-property cache hit: shape compare, slot check
    against the cell size);
  - `call` (frame build, entry into a compiled callee with `call_indirect`);
  - `return`/`return_undef`.

  Blocks are laid out in pc order inside `loop` + `br_table`, so the function can be
  entered at any leader. Forward jumps are `br`, backward jumps go through the dispatch.
  It has four variants:
  - `mem`: every operand in memory, as the interpreter keeps them, so only dispatch is
    removed;
  - `reg`: locals and operand stack in wasm locals, flushed at exits;
  - `reg-inline`: `reg` with the call sequence inline instead of in a module helper;
  - `reg-nofuel`: `reg` without fuel, to measure what exact fuel costs.

**What is faithful and what is not.** The bytecode, frame layout, value encoding, fuel
rule and exit protocol are djs's. The object model is simplified:
- the cache is a (shape, slot) pair;
- djs's has kinds, prototype holders and epochs (D1620, D1621), which a real JIT would
  call or partly inline;
- there is no arena base register and no bounds check.

So these numbers are an upper bound for a baseline JIT: optimistic by an unknown amount
on property code, and close on arithmetic, branches and calls.

### Correctness: the interpreter as the oracle

`node test.ts` runs every program interpreted and in the `reg`, `mem` and `reg-inline`
variants, in fuel slices of 7, 97, 1000 and 1e9. All 60 runs match the interpreter:
- the result;
- the number of slices;
- after every slice, the registers, live stack slots, heap and caches.

Two deliberate bugs both fail it:
- one block's fuel off by one: 34 failures;
- one slot not written back before a call: failures from the first slice on.

### Speed against djs's interpreter

The same functions run in djs-wasm through the Sandbox and in the prototype, both with
fuel slices of 10,000 (pi's). djs times are the median of 5 runs after a warm-up, minus
an empty run. The JIT time is time inside compiled code: it leaves out the miniature
interpreter, which is TypeScript and steps the few instructions around each fuel stop.
Median of 5 runs, with the modules compiled once and instantiated per run:

| program | djs interpreter | JIT `mem` | JIT `reg` | JIT `reg`, no fuel | speed-up (`reg`) | `reg`, Liftoff only |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| tight: `s += i`, 10 M | 148.5 ms | 52.1 | 42.5 | 37.4 | 3.5x | 143.5 (1.0x) |
| loop: `sum += i*3 - (i>>1)`, 5 M, + countdown | 161.6 | 45.6 | 35.8 | 33.5 | 4.5x | 91.2 (1.8x) |
| call1: `s += f(i)`, 3 M | 93.2 | 27.9 | 28.8 | 27.8 | 3.2x | 130.8 (0.7x) |
| fib(25) | 7.4 | 2.2 | 2.1 | 1.9 | 3.5x | 8.7 (0.85x) |
| props: 7 field accesses per iteration, 1 M | 59.6 | 9.2 | 7.5 | 6.9 | 7.9x | 19.6 (3.0x) |

What this shows:
- **Removing dispatch is most of the gain.** `mem` keeps every value in memory, like
  the interpreter, and gets 2.8–6.5x. Wasm locals add about 20% on loops and nothing on
  calls.
- **Exact fuel costs 4–14%** in compiled code: `reg` against `reg-nofuel`.
- **Outlining the call sequence** into a module helper shrinks call-heavy functions by
  a third (fib: 2,837 → 1,490 bytes), at no measurable speed cost (`reg-inline` is
  within noise of `reg`).
- **Liftoff code is about interpreter speed.** TurboFan-compiled `run_hot` is a
  well-tuned loop. Liftoff code of a boxed-value template is not. The gain needs
  TurboFan.
- **In per-operation terms**, djs's interpreter takes about 1.7 ns per bytecode on
  tight, compiled code about 0.5 ns. A call and return take 30 ns interpreted (fib) and
  8.6 ns compiled, with the frame still built in memory. For scale: V8 runs tight as
  plain JavaScript in 40 ms on this machine, about as fast as the compiled template
  (`s` overflows int32 early and stays a double).

### V8 compile costs

The modules of the `reg` variant. "Function" is the compiled template's code, and each
module adds about 1 KB of helpers (float paths, call entry, return). Per module, median
of 12 compiles of unique bytes (V8 caches modules by their bytes):

| template | bytecode | function | module | `new Module` (lazy, default) | first call (Liftoff compile) | eager Liftoff | eager TurboFan |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| id | 5 B | 345 B | 1,386 B | 0.026 ms | 0.010 ms | 0.074 ms | 0.31 ms |
| tight | 45 | 879 | 1,920 | 0.030 | 0.023 | 0.100 | 0.68 |
| call1 | 52 | 1,137 | 2,178 | 0.025 | 0.024 | 0.097 | 1.26 |
| fib | 44 | 1,490 | 2,531 | 0.049 | 0.043 | 0.103 | 1.90 |
| loop | 80 | 1,645 | 2,686 | 0.030 | 0.033 | 0.089 | 1.35 |
| props | 118 | 2,261 | 3,302 | 0.026 | 0.038 | 0.115 | 1.52 |
| big10 (synthetic) | 598 | 14,447 | 15,488 | 0.061 | 0.26 | 0.32 | 7.0 |
| big100 | 5,638 | 137,158 | 138,201 | 0.35 | 2.4 | 2.7 | 134 |
| big1000 | 56,038 | 1,396,824 | 1,397,867 | 2.7 | 23.8 | 29.6 | zone OOM |

Instantiation takes 3–40 µs.

- **Code grows 20–34x from bytecode to wasm.** djs bytecode is dense, at 1–7 bytes per
  instruction. A template expands to about 50 wasm bytes per instruction (tight: 17
  instructions, 879 bytes), mostly type checks and exits.
- **TurboFan costs 0.5–1 ms per KB of wasm**, worse than linearly on large functions.
  It runs on a background thread, but it is CPU the pi process pays for. Liftoff
  compiles at about 50 MB/s.
- **Many small modules**: 1,000 one-function modules cost about 127 µs each on the main
  thread (bytes, compile, instantiate, first call) and about 23 KiB of RSS each.

### Break-even

A new script's code has never been compiled, and V8 has no cached code for it. The
prototype compiles and instantiates fresh modules (V8 time only, not the TypeScript
emitter) and runs the function, against djs-wasm running it (medians of 10):

| program | n | djs interpreter | JIT, total | of which V8 compile + instantiate |
| --- | ---: | ---: | ---: | ---: |
| tight | 1,000 | 0.032 ms | 0.149 ms | 0.094 ms |
| tight | 10,000 | 0.169 | 0.220 | 0.100 |
| tight | 100,000 | 1.47 | 0.92 | 0.086 |
| tight | 1,000,000 | 14.9 | 5.35 | 0.096 |
| call1 | 1,000 | 0.056 | 0.184 | 0.084 |
| call1 | 10,000 | 0.329 | 0.521 | 0.092 |
| call1 | 100,000 | 3.04 | 2.25 | 0.095 |
| call1 | 1,000,000 | 30.7 | 12.0 | 0.090 |

The break-even is between 10^4 and 10^5 iterations, 0.2–0.5 ms of interpreted time in
one function. Below that the JIT is slower. At 10^5 the code is still mostly Liftoff
(1.35–1.6x), and only at 10^6 does it approach the TurboFan speed-up. The JIT column
includes the prototype's TypeScript interpreter for the few instructions around each fuel
stop, so it is slightly pessimistic for small n. A harvested script
spends a median of 0.12 ms on everything: compiling its source, running, and calling its
tools. This matches pico's own measurement: with its JIT "on", fib(25) takes 8.27 ms
against 5.40 ms interpreted, because installation costs more than it returns.

## 5. Costs

- **Engine code.** An emitter covering `run_hot`'s ~70 fast instructions needs analysis
  (leaders, stack depths, resume points), calls and returns, the inline-cache kinds, and
  the `Each`/native-caller interplay. The prototype has 26 opcodes in 500 lines of
  TypeScript, so this is an estimated 3–5k lines of Rust. That is +40–80 KB of raw
  wasm, as much as 15–30 more `@native` builtins against the gzip budget. On top come
  the djs-wasm exports, the exported growable table, and a host driver in sandbox.ts
  with asynchronous compiles, a module cache and slice scheduling.
- **Memory.** Per VM: a compiled-code slot per template (outside the image) and a few
  counters. In V8: about 23 KiB of RSS per module on Liftoff, more with TurboFan, cached
  per process.
- **Complexity and risk.** Every fast path exists twice: in `run_hot` and in the
  emitter. The two can diverge, and only the identity oracle catches it. D1352's lesson
  applies here too: register allocation in TurboFan-compiled code is fragile, so each
  template needs measuring. A JIT bug corrupts memory without a bounds check to stop it.
- **Testing.** The oracle is strong and cheap to state. Run the whole corpus, e2e and
  the active test262 group with the JIT forced on: every function compiled up front, as
  pico's `force`/`stress` modes do. Compare against the interpreter in slices of 7, 97,
  1000 and infinity, with and without restoring into a fresh VM after every slice. That
  roughly doubles the gate's corpus time, and the image comparison needs the dead slots
  cleared (section 3).
- **0151 (denser bytecode, L4) is no blocker.** The prototype reads the disassembly,
  not the encoding. A real emitter would decode through the generated decoder, so new
  operand widths only change the decoder, and new fused instructions add templates.

## 6. Recommendation

1. **No runtime JIT of user code for codemode.** Codemode scripts are 0.1–1.4 ms in
   total, far below a function's break-even and V8's tier-up. Their heavy work is leaf
   Rust (JSON, strings, regexp, allocation, GC), and the tools' latency dominates
   everything. On pi-like, the best case among the workloads, the ceiling is about 10%.
2. **AOT stays `@native`, one builtin at a time, each justified by a benchmark.** That
   is the "builtins become more wasm" path, and it already exists. It gives 2–3.5x on
   builtin-heavy code, compiled by LLVM rather than by a template.
3. **Revisit a bgen wasm backend in a side module** only if the module-size budget
   blocks `@native` additions that measurements justify. Its ABI (intrinsics as exports,
   native frames across modules) is the real cost, and its payoff is limited to
   builtins that are bytecode today.
4. **If djs is ever used for compute-heavy scripts** (> 10 ms of interpreted time in hot
   functions), build the prototype's variant:
   - a baseline template JIT, host-driven like pico's (djs-wasm stays import-free,
     compiles happen between slices);
   - off by default;
   - exits only at first, no helpers;
   - entered through a trampoline, functions capped in size;
   - the interpreter as the oracle in every stress mode.

   Expect 3–4.5x on interpreter-bound loops after V8 tier-up, nothing before it.
5. **Cheaper wins in the same class stay in the interpreter.** Dispatch is what a JIT
   removes, and wasm has no computed goto. The levers on dispatch are fewer and denser
   instructions (0151's L4) and fused instructions such as compare-and-branch.

## Reproducing

From `work/scratch/0153/proto` (branch `scratch/0153-wasm-jit-proto`). It needs a release
`djs` (`target/release/djs`) and the perf wasm build at
`work/scratch/0153/perf/target/wasm32-unknown-unknown/wasm/djs_wasm.wasm`
(`scripts/perf/setup.sh` with `DJS_PERF_DIR=work/scratch/0153/perf`):

```sh
node test.ts                                   # the oracle: 60 runs
node bench.ts run                              # compiled code, all variants
MODES=reg node --liftoff-only bench.ts run     # Liftoff only
node bench.ts djs                              # the same functions in djs-wasm
node bench.ts compile [--big]                  # compile costs (add V8 flags:
                                               #  --no-wasm-lazy-compilation,
                                               #  --no-liftoff --no-wasm-lazy-compilation)
node bench.ts short                            # break-even
node --expose-gc manymods.ts 1000              # many small modules
```

Profiles: `work/scratch/0153/prof/buckets.py *.cpuprofile`. Harvest timing:
`work/scratch/0153/harvest/run.ts`.
