# djs: Architecture

Status: this started as the proposal written before any code. It now describes djs as
built (November 2026, after issue 0095's review). The original reasoning is kept; where the
implementation went another way, the text says what is and cites the decision
(`Dnnnn` in docs/decisions.md, by topic in docs/decisions-index.md). Plans that were
dropped or are not built yet are marked **Superseded** or **Not built**. README.md has the
measured status and how to build and test.

djs is a small JavaScript + TypeScript interpreter written in `no_std` Rust (with `alloc`).
It is primarily meant to be compiled to WebAssembly (wasm32 is the target that memory and
performance design is judged by, D016) and hosted from JavaScript (pi's
codemode), replacing QuickJS / dquickts there. Behavior should roughly match
`~/Development/dquickts`, which is also the test oracle.

## Goals

In priority order. Each design decision below should be traceable to one of these.

- **G1. Cheap data passing** between the sandbox and the host JS. Tool results (often large
  strings, arrays of similar objects, base64 images) go in, and text, JSON and images come out.
- **G2. Fast startup, compilation and execution as a wasm build.** A codemode script should go
  from source text to running in well under a millisecond plus its own runtime. The wasm
  binary should be small.
- **G3. No AST.** A single-pass compiler goes from tokens to bytecode, like QuickJS.
- **G4. Persistence.** A session waiting on host calls (its data, functions, suspended
  activations, open calls) can be written as a continuation dump, a plain Script that any
  later build (or node) runs to continue it (D2560, D2561, D2830, docs/continuation-dumps.md). Inside a
  process the whole VM is a heap image (startup from the base image, stress checks).
- **G5. Suspend anywhere.** Execution can stop between any two bytecode instructions,
  including inside getters, `toString` calls, `Array.prototype.map` callbacks, promise jobs,
  generators and async functions, and continue later in the same process. Across processes
  a run continues from its idle points (D2561).

Non-goals for now: Intl, Temporal, `SharedArrayBuffer`/Atomics, workers, sloppy mode,
`with`, direct `eval`, a JIT, and full test262 conformance for its own sake. ES modules
exist for embedders (D3171, D3190): a host loads a graph through its loader; codemode and
the shipped module have none (`import()` compiles and rejects, D1582).

D016 (owner direction) adds the measuring stick: wasm32 is the primary target, so memory
and performance work is judged by its effect in wasm32 under node/V8. Layouts are designed
for a 32-bit address space, memory grows in 64 KiB pages, and code size counts.

## The Core Idea

**All VM state lives in one self-managed, position-independent heap: a single contiguous
byte arena addressed by `u32` offsets.**

That covers objects, strings, shapes and property tables, bytecode, function templates, the
JS value stack, call frames, the job queue, the pending host call table, the host inbox,
atoms, and realm intrinsics. Rust data structures (`Vec`, `Box`, …) hold only transient
state: the compiler's working state, and scratch space inside a single native operation.
djs-core uses no BTree collections and no core slice sorts, for code size (D1432).

Each goal follows from this:

- **G4:** within a process, an image of the VM is a compacting GC followed by a copy of
  the arena bytes. The durable form, the continuation dump, is written by walking that
  same heap at an idle point (D2560, D2561); images never leave the process (D2700).
- **G5:** the interpreter never recurses on the Rust stack for JS calls. Every frame is in
  the arena, so returning from `run()` at any instruction boundary leaves a complete,
  resumable state.
- **G1:** the host can read sandbox values (especially strings) straight out of wasm linear
  memory without a serialization step, because the heap layout is a documented format.
- **G2:** builtins and the realm are set up once at build time and shipped as a base
  snapshot. Creating a VM means copying that snapshot into a fresh arena. Builtins written
  in JS are precompiled into it, so we pay no compile cost for them at runtime.

Native code is never referenced by pointer from the heap. Native builtins are stored as `u16`
indices into a static table, and host functions as host-assigned `u32` ids. This is what
makes the heap position independent and images possible. It works the same way as
dquickts' native registry, but by construction instead of by fixup.

Accessing the heap through bounds-checked offsets also hardens the sandbox: a VM bug or a
corrupted image can corrupt the JS heap or cause a panic (a wasm trap), but it cannot
cause Rust UB. The tools and stress modes verify every image they load (D1056); the wasm
module loads only images made in its own process and checks their header and dependency
descriptor (D2700). In wasm every access is bounds-checked against linear memory as well,
so the arena checks are a second check (D016). The shipped and slim wasm modules
therefore leave them out (`unsafe-heap`, D2520, D3100): there an engine bug is UB confined
to the instance's linear memory, which may corrupt that VM instead of trapping but cannot
reach the host. Native builds, the dump module and all test builds keep the checks.

### Relationship to piccolo

[piccolo](https://github.com/kyren/piccolo) is the model for the **execution side**: a
stackless trampolined interpreter, fuel, and host callbacks that the executor polls instead
of calling recursively. We do **not** adopt its **memory side** (`gc-arena`). There, `Gc`
values are real pointers to Rust-typed allocations, which works against G1 (the host cannot
read the heap) and G4/G5 (copying or suspending a VM would need a full graph serializer). We trade some type safety
for an arena whose bytes are the state.

## Crate / Repo Layout

```
djs/
  crates/
    djs-core/       no_std + alloc. lexer, compiler, heap, gc, interpreter, leaf
                    intrinsics, JSON, regex, BigInt, images. No platform dependencies;
                    always builds for wasm32-unknown-unknown.
                    src/builtins/gen/natives.rs is checked-in Rust generated by djs-bgen.
    djs-bgen/       build-time builtin compiler (std, its own parser for the subset,
                    D1170): checks the builtin TS subset, emits the prelude JS for every
                    builtin and Rust state machines for `@native` ones.
    djs-prelude/    build.rs compiles bgen's JS output with djs-core and emits the base
                    snapshot (include_bytes!, D1321).
    djs-wasm/       wasm32-unknown-unknown cdylib: the integer ABI (D1230), std with its
                    dlmalloc (D1231), panic hook.
    djs-cli/        std binary `djs`: run, repl, disasm, strip-types (docs/cli.md).
    djs-test262/    the test262 runner (qjs subprocess or djs in process).
    djs-difftest/   the differential corpus runner and the djs-abidrive driver.
    djs-bench/      benchmarks: component, engine track, wasm size.
    djs-docs/       the guide's generator and checker (docs/guide, D2100).
  builtins/         builtin sources in the strict TS subset (*.ts), intrinsics.d.ts,
                    tsconfig.json (so tsc and editors type-check them); gen/ holds the
                    generated prelude.js and resume-points.txt.
  js/               TypeScript host package: src/ (Sandbox, wire codec, heap reader,
                    layout.generated.ts), pi/ (the pi codemode adapter), bench/, test/.
  tests/            corpus/ (difftest cases with qjs-generated .expected files),
                    test262/ (ratchet and oracle lists), test262.toml (grouping), bench/,
                    pi/, data/, scores/, strip-types/.
  scripts/          gate.sh, merge-to-main.sh, bgen.sh, regen.sh, build-wasm.sh, bench.sh,
                    timeout.sh, perf/ (instruction-count tools), determinism/, ...
  docs/             compiler.md, objects.md, host.md, wire.md, cli.md, decisions.md and
                    decisions-index.md; guide/ is the guide for humans (D2100).
  work/             issues, the board and the process (work/README.md).
```

The generated files are checked in so that they are reviewable in diffs and the normal
build does not depend on the builtin compiler. `scripts/bgen.sh` regenerates them,
`scripts/regen.sh` regenerates everything generated after a rebase (D1791), and the gate
checks that they are up to date.

Dependencies stay near zero to keep the wasm small and compile times short: djs-core has no
third-party crate. Its math functions (`pow`, `exp`, `sin`, ...: `core` has none) are libm
0.2.16's, copied into the workspace crate `djs-libm` (D3174, which supersedes D1150's
dependency; D1750's build hash covers djs-libm's sources), and djs-wasm uses std's dlmalloc. We wrote our own
number formatting (D1040, D1262) and float parsing, regex (D1111), BigInt arithmetic
(D1461), Unicode tables (D1031) and hashing. Tool crates may use small, well-known crates.

## Values

Values are 64-bit and NaN-boxed (D1050):

- Any non-NaN-boxed bit pattern is an `f64`.
- Tagged values: `undefined`, `null`, `bool`, `int32` (a Number in int32 form, with fast
  paths in the interpreter), the hole and uninitialized (TDZ) markers, and references:
  string, object, symbol, bigint (always a heap cell, compared by value, D1460), host ref
  (reserved), and internal references to non-JS cells such as arrays of values (D007).
  A reference is a `u32` arena offset. 64-bit values are native i64 operations in wasm
  (D016).

A `u32` offset is enough on wasm32. If arenas larger than 4 GiB ever matter, offsets can be
scaled by the 8-byte alignment.

## Heap

- **Layout:** each cell starts with a header word (cell kind, GC bits, size; D1051). One
  layout description (a macro, D1052) generates the Rust accessors and, through a test,
  the host's `js/src/layout.generated.ts` (D1210). Object cells hold a class, a prototype
  ref, a property storage ref, an elements ref and class-specific slots. Strings are
  separate cells.
- **Strings:** JS semantics are WTF-16 (UTF-16 code units, lone surrogates allowed).
  Strings are stored like V8 stores them: **one-byte (Latin-1)** or **two-byte (UTF-16LE)**,
  plus **ropes** for concatenation (codemode builds output with `+=` a lot). Ropes are
  flattened lazily. Append chains (`s += piece` once `s` is a few hundred units long) use
  **append ropes** over a growable flat buffer that is written in place, so the live
  string is one buffer instead of a rope node per append (D1610); short concatenations
  allocate nothing on the Rust side (D1871). Atoms (interned strings used as property
  keys, symbols included) live in a heap hash table (D1054).
- **Objects:** named properties are in one of two modes (0083, docs/objects.md 7). In
  **shape mode** an object holds only a values block, and a shared **shape** (hidden
  class) lists the keys and attributes in insertion order. Shapes form a tree by property
  additions; transitions are strong and the tree is bounded by a per-heap shape budget,
  so it never depends on when collections ran (D1831). In **dictionary mode** (more than
  32 keys, attribute changes, index keys, builtin namespaces and prototypes, the budget
  spent) an object owns a property table: linear up to 8 entries, hashed above. Property
  entries are 16 bytes and accessors live in an AccessorPair cell (D1830). Arrays get a
  dense elements store; index properties that are not plain go to the table (a flag on
  the object keeps the dense paths' checks cheap). Map and Set are ordered hash tables
  rehashed by the collector (D1381); WeakMap, WeakSet, WeakRef and FinalizationRegistry
  use ephemeron tables (D1382, D1513).
- **Code:** function templates (bytecode, constants, line table, scope info) are heap cells.
  That way compiled code is part of images: the base image carries the builtins' code,
  and pi's prelude image carries its compiled prelude (D1495).
- **GC:** two collectors over one layout-driven trace (D1090, D2040).
  - **In-VM collections slide in place** (mark-compact, D018/D2040). The four passes are
    mark, block table, update and slide. The side tables are allocated up front: a
    live-word bitmap and a block table, about 2.3% of the arena. There is no to-space, so
    a collection's peak is the arena itself. Survivors keep their allocation order.
  - **An old region** `[0, Roots::OLD_END)` (D2041): the cells below it are live for
    **minor** collections, which neither mark nor move them. A linear walk over the old
    cells finds every reference to a young cell (no write barrier); those slots are the
    roots of the young cells and the only old slots the update pass rewrites. The old end
    is the base image's length after `vm_new`, and `Vm::seal` (pi calls it after its
    prelude) sets it to the arena size after a full collection. Full collections shrink
    the old region to its live cells but never extend it (D2042). The safepoints'
    collections are minor, and every 16th is full, as is the first one after a canonical
    image (D2045); the schedule is image state, like the
    threshold. Dead old cells, and young cells only they reach, wait for a full collection.
    No card table (D2044).
  - **Cheney copying** (breadth-first, a function of the graph alone) is
    `collect_canonical`. It makes the canonical hash (on a copy, D1730) and the base image,
    and leaves a sealed heap.
  - A full collection keeps the same cells as Cheney, so live sizes and collection points
    don't depend on which collector runs. **Allocation never collects**:
  it bumps (growing the arena) and sets `gc_pending` once the threshold is crossed; the
  collection runs at the next safepoint (an instruction boundary, a frame entry, a
  backward jump, a native builtin's state transition). Native Rust code therefore keeps
  raw values in locals safely.
  - The threshold is part of the image (D1601), in whole 64 KiB pages, and the arena stops
    growing at it (D1870), so collection points are a function of the program and never of
    the fuel slicing (D1710). This is what makes images deterministic.
  - **The memory cap:** the arena has a limit (the embedder's cap minus a reserve of
    `min(cap/32, 1 MiB)`, D1600). Reaching it is the catchable, preallocated
    `InternalError: out of memory`; the handler starts with a collection and may use the
    reserve. Before an OOM reaches JS, the failing step collects and runs once more: an
    instruction that allocates before it changes anything (D1700), the corresponding
    operations of native builtins (D1900), and `@fast` entries, which decline so the
    builtin's path retries (D1910). A refusal by the host allocator below the cap is
    different: it stops the run with `Status::HostOutOfMemory` (D1690).
  - **Not built:** a non-moving large-object space for big strings and buffers; a 1 MiB
    tool output still slides when garbage lies below it (0081 F6). Promotion of long-lived
    young cells into the old region (D2042, D2046) and card marking (D2044) are not built.
    Peak wasm memory is a chaotic function of where dlmalloc places the growing arena, so
    memory claims compare distributions over perturbed runs (D2047, D2048).
- **Roots:** the realm, the value stack and frames, the job queue, the pending host call
  table, the host inbox (D1670), unhandled rejections, and a host handle table (values the
  embedder holds, D1091). Every root lives in the arena.

Because the arena may grow (and so reallocate), the host must never cache raw pointers
across VM calls. The JS side re-derives its typed-array views after every call, which is
standard wasm practice anyway.

## Compiler (no AST)

The compiler is a recursive-descent parser that emits bytecode directly as it parses, the
way QuickJS does. This needs four techniques:

1. **Token-level lookahead scanning** for ambiguous constructs:
   - An arrow function head `(a, {b}) =>` versus a parenthesized expression: skip the
     balanced parens at token level and check for `=>`.
   - A destructuring assignment `[a, b] = x` / `({a} = x)` versus a literal.
   - TS generic calls `f<T>(x)` versus comparisons: speculatively skip a type argument list
     and check what follows, using tsc's rules.

   The scan only moves the lexer; nothing is emitted.
2. **Rewind:** speculation never emits bytecode. A checkpoint is just the lexer state plus
   the current token, so rewinding is a copy (D1080). The only rewrite of emitted code is
   turning the last instruction (a load) into an assignment target.
3. **Deferred variable resolution:** the compiler emits scope-generic opcodes
   (`get_name atom, scope`) and runs a **resolution pass** once per compilation unit after
   parsing, over all functions, children before parents (D1081). Resolving when a function
   ends is not enough because of hoisting. By then every inner function has reported which
   outer bindings it captures. The pass rewrites each access into a local slot, a
   captured cell (box), or a global lookup. Instructions are encoded so this can be done in
   place, or with one linear re-emit. Hoisting (`var`, function declarations) and TDZ checks
   are handled in the same pass.
4. **Label / jump fixups** are patched when a block closes.

The compiler itself recurses on the Rust stack. That is acceptable because compilation is
bounded by input size and is not something we need to suspend. A nesting-depth limit turns
pathological input into a SyntaxError instead of a stack overflow.

**TypeScript** is erased during parsing. A *type skipper* consumes type annotations, type
parameter and argument lists, `as`/`satisfies`, `!`, `interface`, `type`, `declare`,
overload signatures, and `import type`. Constructs with runtime meaning (enums and
namespaces as closures, const enums folded, parameter properties; D1482-D1484) compile
directly to bytecode. Decorators follow qjs (D1650), and TypeScript's legacy decorators
are behind a compile flag (D1651). Semantics follow tsc with `isolatedModules` and
`useDefineForClassFields`, matching dquickts. There is no separate TS mode in the core:
the flag only enables the TS grammar (D1480), and `djs strip-types` shows the erasure,
blanked like ts-blank-space (D1481).

The **bytecode** is stack based, with variable-length instructions (a `u8` opcode and
immediates), declared once in a table that generates the enum, the emitter, the decoder
and the disassembler (D1082, D1130). A stack VM is the natural target for a single-pass
compiler. A register VM would need either an AST or a much smarter single-pass
allocator. There are no general superinstructions; the compiler rotates loops (the
condition at the bottom), applies peepholes and emits `inc_loc`/`dec_loc` for `i++`
statements (D1860). Function templates, bytecode and constants are heap cells (D1131);
privileged code (the prelude) keeps no line tables or local names (D1430), and string
constants are shared across a compilation unit (D1434).

**Semantics stance:** always strict. Codemode scripts compile as the body of an async
function (D008, D1410). A Script mode (global `var`/function bindings on the global
object, a global lexical scope, completion values) exists as well, for test262,
`$262.evalScript` and the REPL (D004, D1581). Module code (the `modules` feature, on in
native builds, D3171) is a goal of its own: `import`/`export`, top-level bindings in a
module environment set up by linking, `this` undefined, no top-level `await` yet; the host
loads module graphs through a loader callback, synchronously, with no I/O in the engine
(vm/module.rs, D3190). `import()` still compiles and rejects (D1582). There is no direct
`eval`. Indirect eval and the `Function`
constructors exist (D014, D1590, D1591): a call `eval(x)` behaves as indirect eval.
Functions created by the `Function` constructors without a `"use strict"` directive get
sloppy `this` binding only (D015, D1630), nothing else of sloppy mode. `using` and
`await using` are supported (D1580).

## Interpreter

```
loop {
    if fuel == 0 { return Status::OutOfFuel }
    decode op at pc; execute; fuel -= cost(op)
}
```

- **Stackless:** a JS → JS call pushes a frame record onto the arena's value stack and
  continues the loop. Returning pops it. The Rust stack depth stays constant.
- **Re-entrant operations without recursion:** an opcode that may invoke user code (getter,
  setter, `valueOf`/`toString` during `+`, iterator protocol in `for-of` / spread /
  destructuring, `instanceof` with `Symbol.hasInstance`, Proxy traps, D1450) pushes a frame
  for the callee with one of a few fixed *continuations* (push the result, discard it,
  the construct result rule, return from `run`). **An op never resumes mid-way** (D1085):
  it runs its fast path inline, and when user code is needed it either calls the user
  function directly (getters, setters) or calls a privileged JS builtin (`%Add`,
  `%ToPropertyKey`, `%GetIterator`, …) that holds the multi-step logic. Nothing is left on
  the Rust stack, and the interpreter has no per-op state machines. (Generated `@native`
  builtins do have states; see "Builtins".) A slow path that needs a conversion first
  replaces the operand and runs the instruction again (the Retry continuation, D1280,
  D1861).
- **Fast paths:** the main loop is the tier-0 loop `run_hot` (D1930). It keeps `pc`,
  `sp`, the fuel and the frame data in Rust locals and runs the hottest simple ops itself
  (locals, constants, int32 and float arithmetic, comparisons, jumps, calls and returns
  of JS closures, inline-cache hits, dense elements; D1931). Anything else, and every
  case it declines, goes to the full `step` (`step_once`) with identical fuel and
  safepoints. It replaced the earlier `fast_step`/`fast_more` (D1350, D1352, D1783,
  D1860), which remain only in tests as the reference it must equal. Register allocation
  in wasm is sensitive, so every addition is measured (D1352, D1862). Property access uses inline caches: verified table indexes per
  template, and prototype caches validated by an epoch (D1620, D1621). for-of and
  array destructuring step Array, String, Map and Set iterators without result objects
  while `next` is the original (`iter_next`, D1800).
- **Exceptions:** per-function handler tables map pc ranges to handlers. `finally` is
  entered with a jump, its completion (kind and value) in two locals (D1084, D2733). Unwinding walks frames in the arena.
  Stack overflow is a RangeError (D013, D1540); errors carry qjs-style stack traces
  (D1282).
- **Generators and async functions:** on `yield`/`await`, the frame's slice of the value
  stack is copied into a heap generator/async object (D1375). Resuming copies it back.
  Closures capture boxed cells, chosen by the resolution pass, so frames never need to
  outlive their stack slot. Privileged prelude functions refer to each other through the
  realm's prelude function table (`get_builtin`/`put_builtin`, D2440), not through boxes.
- **Promises and jobs:** the microtask queue is a heap structure (D1370, D1371). Running a
  job means pushing a frame. Results of async host calls wait in the host inbox and are
  applied one at a time when the job queue is empty (D1670).
- **Fuel:** a counter decremented per instruction, weighted by cost. Native ops charge fuel
  in proportion to the work they do (bytes copied, elements visited); `@fast` entries
  cost a flat 2 (D1850). Running out of fuel is just another suspension. The embedder
  turns it into a timeout or a time slice. An instruction's overshoot
  carries to the next run as a debt in the heap, so a budget given in different slices
  stops at the same instruction (D1740, D1711, D1712).

### Builtins

#### The leaf rule

The rule that makes G5 hold for builtins:

> **Hand-written Rust never calls back into JS.** Hand-written Rust is limited to *leaf
> intrinsics* (`%ArrayPush`, `%StringIndexOf`, `%ToPropertyKeyFast`, …), which never run
> user code. Every builtin that may run user code is written in the **builtin language**
> (below) and runs either as bytecode or as a generated, resumable Rust state machine.

- Builtins that may run user code: `Array.prototype.{map,filter,forEach,reduce,find,some,
  every,sort,…}`, `Promise` combinators, `JSON.stringify` with replacer, `toJSON` or
  accessors, `JSON.parse` with reviver, `String.prototype.replace` with a function,
  iterator helpers, `Array.from`, `Object.fromEntries`, and so on. Almost any builtin that
  takes an object qualifies, because `ToPrimitive`, getters and (later) proxies can run
  user code.
- Leaf fast paths stay leaf. A leaf `JSON.stringify` handles the common case (plain data
  objects with no accessors, no `toJSON`, no proxies) and returns a "needs slow path"
  signal that falls back to the full version. `Array.prototype.sort` on primitives without
  a comparator can be a leaf.
- Everyday builtins with primitive arguments (`includes`, `split`, `Math.max`,
  `String(x)`, `hasOwnProperty`, the ToString/ToNumber helpers, ...) and the element
  loops of Array builtins over dense arrays (push, pop, indexOf, slice, join, fill, ...)
  are tagged `@fast`: a Rust entry runs in place of the call (in `Exec::call` and in the
  run loop after `run_hot` declines a call, not inside `run_hot`) when nothing
  observable can happen, and declines otherwise, also at the heap cap (D1850, D1910). The
  same rule decides when a builtin may skip a protocol: iteration and spread over
  original iterators (D1351, D1800, D1802), dense-array leaves (D1801), JSON's fast path
  for plain subtrees and Dates (D1901).
- Long leaf ops (for example a regex scan, `JSON.parse` of 10 MB, a big `join`) are
  bounded and fuel-charged. If they turn out to cause suspension latency problems, they
  get an explicit resumable state stored in the heap, as the regex engine has from the
  start (see below).

#### The builtin language

Builtins are written in a **strict subset of TypeScript**, in the spirit of V8's Torque.
One source has two backends:

1. **Bytecode backend (every builtin).** djs-bgen erases types and emits plain JS with
   `%Intrinsic` calls. djs compiles it in privileged mode into the base snapshot (D1172).
   Privileged mode unlocks `%Name(...)` intrinsic calls (`call_intrinsic`, D1270); the
   proposal's privileged-only opcodes (unchecked element access, int32 loop counters,
   direct slot access) were not needed: the leaf intrinsics and the interpreter's fast
   paths cover them. Privileged code keeps no line tables or local names (D1430). This
   costs nothing in wasm size.
2. **Rust backend (opt-in, `/** @native */`).** djs-bgen emits a resumable Rust state
   machine (D1632). It is used for hot paths (`map`, `forEach`, `filter`, `join`,
   `indexOf`, `GetIterator`, the Map/Set constructors, the `JSON.stringify` slow path, …)
   and justified by benchmarks, since each one adds wasm size (D1634 keeps the native
   API out of line in wasm; `scripts/native-sizes.sh` prints the size per function).

Because both backends compile the same source, they can be tested against each other
(D1635): the gate runs the corpus with the native backend in every stress mode and once
plainly with the bytecode backend, and `--backend bytecode|native` selects the backend in
`djs run`, djs-difftest and djs-test262. `ExecOptions::native_builtins` picks the backend
per call; fuel counts differ between them, results do not (D1633).

The subset:

- Explicit types: `Value`, `Obj`, `Str`, `Sym`, `Fn`, and the numeric types `i32`, `u32`,
  `f64` (branded `number` types in `intrinsics.d.ts`; tsc catches mixing `i32` and `u32`, and
  bgen is the authority on the full numeric rules, D1171). Numeric semantics must agree
  between the backends. Integer arithmetic is checked, and bgen rejects operations whose
  JS and Rust meanings differ.
- Intrinsics are declared in `intrinsics.d.ts` and tagged either `@leaf` (a plain Rust call
  that may throw but never runs JS) or `@reentrant` (may run user code). A call to a
  reentrant intrinsic, a user function or another reentrant builtin is a **resume point**.
  bgen infers these points; the generated Rust marks each one with a comment.
- Abrupt completions map to `Result` in Rust (the spec's `?`). `throw` and `try`/`catch`/
  `finally` are supported, with the handlers recorded in a per-function table (the Rust
  backend copies `finally` onto every way out, D1803). `@native` constructors work too.
- Not supported: closures over mutable locals, `arguments`, `eval`, dynamic property
  tricks on builtin locals, and generators (iterator builtins are written as explicit state
  objects: iterator helpers D1510, Array.fromAsync D1511, disposable stacks D1512). The
  goal is spec-algorithm style code, nothing more. bgen's rules are in D1171 and
  builtins/README.md; installation tags (`@builtin`, `@constructor`, `@getter`, `@value`,
  `@intrinsic`, `@native`, `@fast`) in D1310 and D1315.

Intrinsics themselves (leaf ones) stay hand-written Rust over the heap accessors.

#### Generated state machines

A `@native` builtin runs in a **native frame** in the arena, next to JS frames. The frame
record is a JS frame's (callee, `this`, arguments, header) with a native bit in the
header; the callee's template holds the native function's index, `pc` holds the state,
and the locals are bgen's slots (D1631). bgen knows every local and its type, so
it lays them out as slots. Every local lives in a frame slot (as a Value), which makes
it a GC root. Because the frame holds no pointers, native frames suspend and travel in
images like JS frames.

The GC and fuel rules that keep this simple:

- **No GC inside a state arm.** Allocation never collects. It bumps (growing the arena if
  needed) and sets a `gc_pending` flag. Collection happens only at state transitions and at
  interpreter safepoints. Inside one arm, Rust locals holding references are therefore safe;
  across arms, only frame slots survive. The one exception is the collect-and-retry at the
  heap cap (D1900): it happens only where the generated code holds no heap value in a Rust
  local except the operation's arguments, which are rooted on the stack first (a state's
  final call, or a call that is a whole statement, which djs-bgen marks).
- **Fuel is charged per transition** (4 per loop back-edge, 2 per call, D1633, and in
  proportion to the work leaf calls do). A loop back-edge is a transition and a
  safepoint, so a `map` over a million elements can run out of fuel and suspend in the
  middle, like bytecode would.
- **Fast paths are tried inline, and the slow path is a resume point.** For example `get`
  first tries a leaf lookup of an own data property. Only when it finds a getter (or a
  proxy) does the frame save its state and push the callee. The common case never leaves
  the arm.

Source:

```ts
/** @native */
export function map(this: Value, callback: Value, thisArg: Value): Obj {
  const o: Obj = toObject(this);
  const len: u32 = lengthOfArrayLike(o);       // reentrant: getter, valueOf
  if (!isCallable(callback)) throw typeError("not a function");
  const a: Obj = arraySpeciesCreate(o, len);   // reentrant: constructor, species
  for (let k: u32 = 0; k < len; k++) {
    if (hasProperty(o, k)) {                   // reentrant: proxy trap
      const v: Value = get(o, k);              // reentrant: getter
      createDataProperty(a, k, call(callback, thisArg, v, k, o));
    }
  }
  return a;
}
```

Generated Rust (shape only, slow-path states abbreviated):

```rust
// Generated by djs-bgen from builtins/array.ts:12 (Array.prototype.map). Do not edit.
pub(crate) fn array_map(vm: &mut Vm, fr: NativeFrame) -> Step {
    loop {
        match fr.state(vm) {
            S0 => {
                let o = intr::to_object(vm, fr.this(vm))?;
                fr.set(vm, O, o);
                // resume point: lengthOfArrayLike(o)
                return fr.call_builtin(vm, S1, LENGTH_OF_ARRAY_LIKE, &[o.into()]);
            }
            S1 => {
                fr.set_u32(vm, LEN, fr.result(vm).as_u32());
                if !intr::is_callable(vm, fr.arg(vm, 0)) {
                    return fr.throw(vm, intr::type_error(vm, "not a function"));
                }
                // resume point: arraySpeciesCreate(o, len)
                return fr.call_builtin(vm, S2, ARRAY_SPECIES_CREATE, &[...]);
            }
            S2 => {
                fr.set(vm, A, fr.result(vm));
                fr.set_u32(vm, K, 0);
                fr.goto(vm, LOOP)?; // transition: fuel + gc safepoint
            }
            LOOP => {
                let k = fr.get_u32(vm, K);
                if k >= fr.get_u32(vm, LEN) {
                    return fr.ret(vm, fr.get(vm, A));
                }
                let o = fr.get(vm, O);
                match intr::get_own_fast(vm, o, k) {
                    Fast::Value(v) => {
                        fr.set(vm, V, v);
                        // resume point: call(callback, thisArg, v, k, o)
                        return fr.call(vm, S_STORE, fr.arg(vm, 0), fr.arg(vm, 1),
                                       &[v, k.into(), o.into()]);
                    }
                    Fast::Absent => { fr.set_u32(vm, K, k + 1); fr.goto(vm, LOOP)?; }
                    Fast::Slow => return fr.call_builtin(vm, S_HAS, HAS_PROPERTY, ...),
                }
            }
            S_STORE => {
                let r = fr.result(vm);
                intr::create_data_property(vm, fr.get(vm, A), fr.get_u32(vm, K), r)?;
                fr.set_u32(vm, K, fr.get_u32(vm, K) + 1);
                fr.goto(vm, LOOP)?;
            }
            // S_HAS, S_GET: the hasProperty / get slow path
            _ => unreachable!(),
        }
    }
}
```

As implemented (0063, D1632; `crates/djs-core/src/builtins/gen/natives.rs`), the arms
call `ex.n_*` helpers of `interp/run/native.rs` instead of a `fr` handle, every intrinsic
call goes through the interpreter's `call_intrinsic` (with inline fast paths that mirror
it), and small internal helpers such as `ToString` are inlined into the caller's states.
`fr.call*` and `fr.ret` return a `Step` to the interpreter loop. The loop pushes the callee
frame (JS or native) and later re-enters `array_map` at the saved state. `?` on a throw
unwinds through the frame's handler table. `fr.goto` returns early (with the state already
saved) when fuel is exhausted or a GC is pending.

On the bytecode backend the same source compiles to ordinary JS frames. A reentrant call
there is just a call, and suspension works as it does for user code.

The **regex** engine is our own backtracking engine (D1111). It compiles to regex bytecode
stored in the heap (literals compile once, D1502), and its backtrack stack is in the
arena. It is fuel-metered and resumable (D1332, D1711), so catastrophic backtracking is a
timeout, not a hang. Unicode property escapes and normalization are built in, from
packed UCD tables (D1500, D1760, D1031); unmodified RegExps take fast paths that skip
the protocol (D1503, D1560).

## Host Interface

### Execution API

The plan was a handful of exports (`vm_new`, `vm_compile`, `vm_call`, `vm_run`,
`vm_resolve_host_call`, `vm_snapshot(vm, base?)`, `vm_restore`; snapshots were later
replaced by continuation dumps, D2560, D2700). What exists is the
integer ABI of djs-wasm (D1230): numbered VMs and handles, a staging buffer for bytes in
and an out buffer for bytes back, and about 45 exports, documented in docs/host.md "The
ABI". In outline:

```
vm_new(memory_limit) -> vm                       // a copy of the base image (D1321)
vm_from_bytes(len, memory_limit) -> vm           // an image of this process, from staging (D2700)
vm_arena_ptr / vm_arena_len                      // the image: the arena bytes
vm_dump / vm_replace_image                       // continuation dumps (the dump module, D2704)
vm_compile(vm, goal, params_len, src_len) -> h   // goal: async body, Script, TS, ...
vm_call(vm, f, this, argc)                       // pushes a frame, does not run it
vm_run(vm, fuel) -> status                       // Done | Threw | OutOfFuel | HostCall |
                                                 // Idle | HostOutOfMemory
vm_host_call_info / vm_resolve_host_call(_at)    // sync and async host calls
vm_decode_wire, vm_parse_json, vm_json_stringify // data in and out (with the reader)
vm_set_now, vm_set_time_zone_offset, vm_set_random_seed, vm_set_weak_refs
vm_collect, vm_canonical_hash, vm_abort, vm_take_unhandled_rejections, ...
```

The JS host package wraps it: `Sandbox` (js/src/sandbox.ts) runs codemode bodies with
host functions, fuel slices, host-driven runs that persist as continuation dumps (with
the dump module beside the shipped one, D2630, D2704), a deterministic mode and journals;
js/pi/vm.ts adapts it to pi's codemode contract (D1490), and in its next mode persists pi
sessions as dumps (D2650).

- **Async host functions** (`tools.*`, `models.*`): calling one creates a promise and an
  entry `{call_id, fn_id, args, promise}` in the heap's pending host call table. Execution
  continues, and the host reads all new pending calls whenever `run` returns. Resolving one
  puts the result into the VM's inbox (heap state); it is applied as a macrotask once the
  job queue is empty, one result at a time (D1670), so behavior depends only on the order
  of results, not on when the host delivers them.
- **Sync host functions** (`text()`, `console.log`, `store`/`load`): the VM returns
  `HostCall` with the frame parked mid-instruction. The host handles it (possibly
  asynchronously on its side), resolves it, and calls `run` again. This is dquickts'
  `JS_SuspendCall` in stackless form, and it means every host call is a suspension point.
- **Durability:** pending host calls are heap state. At an idle point a dump writes each
  open one as `$dump.pending(id, name, args)`; after loading it the host decides whether
  to deliver a recorded result, re-issue the call or reject it (D2560). The VM provides
  the call ids and arguments. The Sandbox also records and replays journals of a run's
  external inputs, checked against canonical heap hashes (D1730).
- **Nondeterminism** comes only from the host: the clock is set by the host (`Date` reads
  it, D1383; with an async result it travels in the inbox entry, D1670), the time zone
  is a fixed offset, `Math.random` is seeded (D1314), and weak references can be made
  strong or absent (deterministic mode, D1680). Same inputs in the same order give the
  same heap images, across fuel slices, restores, and native and wasm builds (D1720,
  D1725; checked in the gate).

### Data passing (G1)

Hot paths in codemode, in order: tool results coming in (strings, sometimes MiBs), text and
console output going out (strings), and structured values both ways (JSON-like; arrays of
similarly shaped objects; base64 images).

- **Outbound, direct read:** the host gets a value handle and reads it with a JS-side
  decoder that understands the heap layout. Strings are decoded in place from linear
  memory: `TextDecoder('utf-16le')` for two-byte strings, a chunked
  `String.fromCharCode` for Latin-1. Note that `TextDecoder('latin1')` is really
  windows-1252 and must not be used. Ropes are flattened first. Objects and arrays are
  walked directly. The VM does no serialization at all.
- **Inbound, wire format:** the host encodes a value into a staging buffer in linear memory,
  and the VM materializes it in one native pass. The format is a compact tagged binary
  encoding (primitives, strings as UTF-8 written with `TextEncoder.encodeInto`, arrays,
  objects with a **key table** so repeated keys are interned once, `Uint8Array`, and
  host refs). The VM transcodes UTF-8 to Latin-1 or UTF-16 natively, which is fast in wasm.
  A lone surrogate cannot survive `encodeInto`; strings that contain one take a UTF-16 path.
- **Baseline to beat:** `JSON.stringify` on the host plus native `JSON.parse` in the VM.
  V8's JSON is very fast, so the binary format has to win in benchmarks to justify itself.
  The wire interface is defined so the encoding can be swapped.
- **Host refs (Not built):** opaque handles to host objects, for pass-through data such as
  image blocks that the script only forwards to `image()`. They would avoid copying
  entirely. In a dump a host ref would be written as an id; the host persists the
  referenced objects or accepts that they are dead after a load. The value tag and the
  wire tag are reserved (docs/wire.md).

As built (D1180, D1210, D1211, D1781; docs/wire.md and docs/host.md have the formats and
the numbers): the wire format beats `JSON.stringify` + `JSON.parse` in wasm by 1.4x on
objects up to 9x on one big string inbound, and the reader by 3x to 130x outbound.

### Images and dumps (G4)

A waiting run persists as a continuation dump (D2560, docs/continuation-dumps.md); heap
images are the in-process mechanism (D2700).

- **Images:** an image is the arena bytes after a compacting GC; the roots and the
  image's **dependency descriptor** live in the arena's root cell (D026, D2265): hashes
  of the cell layouts, the opcode and intrinsic tables and the internal id tables
  (D2266), the image feature set (D2267) and a hand-bumped compatibility version (D2268).
  An image loads into any build with the same descriptor. (Until D026 an image was keyed
  on the engine build hash of D1320/D1330/D1750, which still identifies the build for
  journals.) Images are used for the base image every VM is copied from, pi's prelude
  images (D1495, D1890), the stress and determinism checks, and the transport between
  the shipped module and the dump module (D2635, D2701). The tools and stress modes
  verify every image they load (D1056); the wasm module checks only the header and the
  descriptor, since its images come from its own process (D2700). Everything is
  little-endian, which matches wasm and the usual native targets.
- **Base snapshot:** the initialized realm and the precompiled builtins (about 190 KB,
  compacted before it is collected, D1431). djs-prelude produces it at build time and it
  is embedded in the wasm. `vm_new` copies it into the arena; that copy *is* startup
  (microseconds).
- **Continuation dumps:** at an idle point (no frame, no job, only open host calls,
  D2561) the exporter writes the run as a plain dump (D2830): an ordinary Script that
  recreates its data, each function stopped at an `await` or `yield` as a continuation
  (the rest of its source from there, read by the compiler's suspension maps, D2563,
  D2850), in-flight builtin state as the calls that make it (D2562, D2610), the open
  calls through the host's `$djs.resume` and `Math.random`'s state through `$djs.seed`
  (D2890); `$djs` also holds the dump runtime's helpers (D2903). Only VMs with frozen
  builtins dump (D2836). Any later build, or node, runs it; golden dumps hold that
  promise (D2619, D2920). Hosts store dumps zstd-compressed (D2703).
- **Packaging:** the dumper is in a second module of the same build (packaging B,
  D2564, D2704): the shipped module has none, and a host that persists runs loads the
  dump module and copies the VM's image there to dump, keeping that instance between
  dumps (D2701). Loading a dump is running it, in the shipped module.
- **Determinism:** the GC threshold is in the image (D1601) and collection points do not
  depend on fuel slices (D1710), so the same inputs give the same image; the gate
  compares images across schedules and between native and wasm (D1720, D1725).
  `vm_canonical_hash` hashes the graph independently of collection points (D1730).
- **Deltas and containers: Superseded.** The proposal planned page-level deltas against
  the base (a non-moving old generation, collected only on a full snapshot). 0088's
  research recommended raw images compressed against a reference image instead, which
  was built as the snapshot container (D2260, D2066, D2400): a waiting pi session took 2
  to 8 KB, but restored only into builds of the same descriptor. Continuation dumps
  replaced it for persistence (D2560) and the containers, session images and reference
  compression were removed (D2700). (The GC's old region of D2041 is a collector
  feature, not a format.)
- **Restore of an image:** check, copy the bytes, re-bind host functions by id and native
  builtins by index. There are no fixups because there are no pointers.

## Performance Budget

The initial targets, and where djs stood at 0095 (README.md "Status" has the current
numbers, generated, D2101; D016 says why wasm numbers come first):

| Metric | Target | Measured |
| --- | --- | --- |
| wasm size (release, after `wasm-opt`) | gzip within 1.5x of quickjs-wasi (D020; the original target was < 400 KB raw) | 1,515 KB without wasm-opt (code 935 KB, data 576 KB of which the base image is 398 KB); 531 KB gzip -9, 426 KB brotli |
| `vm_new` from the base snapshot | < 100 µs | about 40 µs in wasm, mostly the copy of the base (D1990); `Sandbox.create` ~60-140 µs, mostly instantiation |
| compile throughput | > 50 MB/s of source on native builds | not tracked as a target; startup of a codemode script is µs |
| interpreter speed | within 1.5x of QuickJS-wasm (stretch: parity) | wasm instructions vs quickjs-wasi: plain loops and property access 0.55-0.65x, strings and arrays 0.55-0.8x, JSON 0.5-0.6x; calls 1.3x, builtin-heavy code (higher-order, forEach) 2-4x, pi-like 1.5x |
| host string out, 1 MiB | ~memcpy speed | yes (the reader, docs/host.md) |

The size gap is mostly the base image and the generated native builtins;
work/issues/0052-bloat-investigation.md has the breakdown, D1634 the rule that each
`@native` builtin must justify its size, and `scripts/wasm-size.py` the per-section view.
The run-time gap on builtin-heavy code is the call protocol and the native state machines'
per-element overhead (work/issues/0081-perf-anatomy.md, the plan behind 0092-0094).

The wasm `Module` is compiled once by the host and cached. Each script gets a fresh instance
or a reset arena.

## Hill-Climbing Strategy

We use **differential testing against dquickts** as the oracle (TESTING.md). Every case runs
through `qjs` (strict mode, async body wrapper, pi's codemode globals) and through djs, and
the transcripts (output, thrown error, return value) must match; error messages match
QuickJS verbatim, because models read them (D1100).

Corpora, as they turned out:

1. **The difftest corpus** (`tests/corpus/`; README.md "Status" has the counts):
   codemode-style scripts with fake tools, console and result formatting, builtins,
   errors, language and TypeScript cases, and dquickts' own `test_*.js` split into cases
   (D1102). Real pi session scripts were not harvested; pi's own tests run on djs instead
   (docs/host.md).
2. **test262**, grouped by `tests/test262.toml` into skip / later / active (D1022); the
   active group (README.md "Status" has the counts) is in the gate with a ratchet of
   known failures.
3. Generated cases (fuzzing): **Not built.**

The loop, as practiced (work/README.md, D012):

- Each issue fixes a failure cluster (`--clusters` groups failures by reason) or a measured
  bottleneck; the ratchet files (`tests/test262/expected-failures.txt`,
  `tests/corpus/expected-failures.txt`) only shrink, and **no regressions** is the merge
  gate (`scripts/gate.sh`, run by `scripts/merge-to-main.sh`).
- The gate runs in one persistent worktree under a merge lock, decides timeouts by fuel
  rather than wall clock, and regenerates generated files after a rebase; agents' runs are
  niced so the gate gets the CPU (D1790-D1793).
- Benchmarks are separate tracks: the engine track (`scripts/bench.sh`, D1190, D1191),
  wasm size (D1192), and instruction counts with `scripts/perf/` (load-independent,
  natively and in wasm).

The **suspension stress mode** (borrowed from dquickts) runs the whole corpus while suspending
every N instructions, restoring into a fresh VM from its image, and resuming (D1161). Output must be
identical. A dump mode writes a continuation dump at every idle point and continues in a
fresh VM that loaded it (D2613). A GC stress mode collects at every safepoint, and a
determinism mode compares heap images across schedules. This turns G4 and G5 from claims
into tested properties.

## Phases

The original phases, with their status:

- **P0, skeleton:** done.
- **P1, core language:** done. The hand-written `prelude.js` was replaced by bgen's output
  in P2 as planned.
- **P2, codemode MVP:** done. pi codemode runs on djs through js/pi/vm.ts (D1490).
- **P3, TypeScript:** done (the type skipper, enums, namespaces, parameter properties,
  decorators).
- **P4, durability:** done, in a different form than planned: deterministic images,
  journals and canonical hashes, and continuation dumps as the persistence format (D2560,
  0169-0178), with images kept inside a process (D2700). Deltas and the snapshot
  containers are **Superseded** (see "Images and dumps").
- **P5, long tail:** done for the listed areas (regex, generators, collections, symbols,
  iterators, typed arrays, BigInt, Proxy, Reflect, stack traces, WeakRef, disposables);
  the test262 active group passes 99.8%.
- **Performance track:** ongoing. The bgen Rust backend (0063), `@native` builtins,
  `@fast` entries, shapes and inline caches, int32 and float fast paths and compiler
  peepholes are in. Open: the run loop and call protocol (0081 P6/P9, 0093), the native
  state machines' per-element cost (0081 P8), a large-object space, host refs.

## Open Questions / Risks

The original list, with what happened:

- **Single-pass + TS ambiguities.** Resolved: checkpoint/rewind (D1080) and deferred
  patterns (D1360) handle arrow heads, generics and destructuring; dquickts'
  `test_typescript.ts` passes.
- **Self-hosting cost.** Real, as feared: bytecode-backend builtins were slow. The answers
  are the Rust backend (`@native`, D1631-D1635), `@fast` leaf entries (D1850, D1910) and
  protocol shortcuts (D1351, D1800-D1802). Builtin-heavy code is still 2-4x quickjs-wasi.
- **Builtin language scope creep.** Held: awkward cases became leaf intrinsics; the subset
  grew only try/finally and constructors in the Rust backend (D1803).
- **Backend divergence.** Both backends run the corpus in the gate and pass the active
  test262 group (D1635); the bgen behavior test compares the bytecode backend with qjs.
- **Copying GC with large strings.** Still open: there is no large-object space. D1870's
  growth policy and append ropes (D1610) limit the damage; large tool outputs are copied
  by every collection.
- **Hand-rolled heap ergonomics.** Resolved as proposed: one layout description (D1052)
  generates the accessors and the JS reader's layout (D1210).
- **Delta snapshots versus a moving GC.** Resolved differently: persistence is by
  continuation dumps, which do not depend on the heap's layout at all (D2560); images,
  in-process only, stay whole (see "Images and dumps").
- **Strictness of matching qjs.** Decided: djs matches QuickJS's messages verbatim
  (D1100, D1113, D1541); deliberate differences are marked per case (`djs-expect`,
  D1540), e.g. stack overflow is a RangeError (D013).
