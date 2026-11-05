# 0218: lazy function compilation, design

Status: design, before building. Measurements: native release, `prof_compile`
(best of n), machine load 7-10, so ratios matter more than the milliseconds.

## Where a compile's time goes today

`compile` is two phases (docs/guide/compiler.md): parse + phase-1 emission of every
function, then `resolve::finish`: classify every name, assign storage, re-emit each
function (peepholes, short forms, stack walk, line table) and write its template.
`sample` on typescript.js (9 MB, one IIFE holding thousands of functions):

| part | share |
| --- | ---: |
| parse + phase-1 emission (`parse_top`) | 38% |
| `resolve::finish` | 60% |
| of which `emit_function` + `alloc_template_in` | ~44% |
| of which classification, `Phase1::finish`, `LineTable::new`, drops | ~16% |

The second half is the part that a never-called function does not need.

## Prototype measurement (thrown away, patch in work/scratch/0218/proto.patch)

`finish` skipped `emit_function` for every function that is not the top level, written
in source (function, arrow, method, getter, setter), not privileged and not an IIFE, and
wrote a stub template instead. The parse stays complete, so all early errors are reported
as now.

| input | eager | deferred | change |
| --- | ---: | ---: | ---: |
| typescript.js | 157 ms | 95 ms | -40% |
| tests/dump/v2/golden/*.dump.js (sum, 131 dumps) | 8.4 ms | 6.2 ms | -26% |
| dump runtime (js/src/dump-runtime.js, 45 helpers) | 0.225 ms | 0.145 ms | -35% |
| codemode harvest (300 real pi scripts, as bodies, sum) | 12.65 ms | 11.4 ms | -10% |
| heap after compiling typescript.js (9 MB of it is source text) | 19.2 MB | 15.5 MB | -36% of the code |

With the deferral, parsing is 63% of what remains (typescript.js).

A function that is called later pays its parse twice: once at load (for the early errors
and the capture analysis), once when it compiles. A function compiled at its first call
costs about 1.45x its eager compile; one never called costs about 0.4x. Lazy wins while
less than about half of the lazy functions' code runs. That holds for dumps (origins,
closures kept for later, helpers) and libraries, not for codemode scripts, which are
mostly top-level code whose callbacks all run: -10% at load on ~40 µs per script, and
some of it given back at the calls.

## Design

### 1. Parse everything, defer phase 2 (no preparser, no kept AST)

The Script's compile runs exactly as now: the full parse with phase-1 emission of every
function, and the full classification. So every early error is reported at load, and the
classification of the whole unit decides, as now, which bindings are captured (boxes),
every function's capture list and its flags (`HOME`, `this`/`arguments` use). The parent
functions' code is byte-identical to today's. Only the re-emit and the template of a
**lazy** function are skipped; it gets a lazy template instead.

At its first call the function's source span is parsed again on its own, as a unit of its
own, and phase 2 runs for it.

Rejected:
- A V8-style preparser: a second grammar next to the emitting parser (size, and every
  syntax change twice). A cheaper variant stays possible later: the same parser with
  phase-1 emission switched off inside lazy subtrees (follow-up; parsing is 63% of the
  remaining time).
- Keeping phase-1 state (code, refs, scopes) for later: it is Rust state, bigger than the
  bytecode, and would not survive images. Keeping it in the heap means a second code
  format. The source is kept anyway (D009): re-parsing it costs nothing to store.

### 2. Which functions are lazy

All of these must hold. The rule only looks at the function's own text and its parent
function's text, so a re-parse of an enclosing lazy function decides the same:
- the unit is not privileged (the prelude stays eager, in the base image);
- the function is written in source: a function declaration or expression, an arrow, an
  object or class method, getter or setter (a mark set by `parse_function_rest`,
  `parse_method_function_dp` and `parse_arrow`). Never lazy: class constructors (default
  ones included), field initializers, static blocks, `accessor` getters and setters,
  decorator thunks, TypeScript enum and namespace closures, the function of a `Function`
  constructor;
- not an IIFE: no call follows it, `fn(`, `(fn)(`; the tokens checked must end inside the
  parent function, so that the parent's re-parse (which ends at its own end) sees them
  too;
- not *pinned*: no reference in its subtree resolves through TypeScript declaration info
  (a const enum folded, a namespace member). Pinning propagates to every ancestor, which
  are then eager too. These are rare and need the unit's `TsDecl`s;
- an arrow whose `this` belongs to a class constructor (directly or through arrows) is
  eager: `super()` and `this` there emit code from the constructor's state (field
  initializer, parameter properties, the derived check);
- a method whose parameters carry legacy decorators (`DecParams`) is eager;
- maybe: a body shorter than N bytes is eager (tiny callbacks; tuned in batch 4 with the
  harvest corpus; N = 0 at first).

### 3. The lazy template

A `FuncTemplate` cell, so closure creation, `name`, `length`, `toString` and the GC need
no change:
- `FLAGS`, `NAME`, `PARAMS`, `LENGTH`, `CAPTURES` and the capture descriptors are what
  the compiled template will have (all known after classification). `LOCALS` = `STACK` =
  0, no handlers, no constants, no inline caches.
- The code is one new instruction, `lazy` (below).
- Each capture descriptor also carries the captured binding's attributes in its free
  bits 16-17: immutable, TDZ-checked (`capture_desc::decode` reads only the low 16 bits
  and bit 31).
- `IC` (a traced `Ref`, unused with `IC_COUNT` = 0): null, then the compiled template.
- `INFO`: the script, `SRC_START`/`SRC_END` as now; `NAMES` = the capture names (the
  "locals, then captures" layout with 0 locals, which the dumper already reads);
  `CHILD_COUNT` = 0; `LINES` = the lazy record, encoded as a line table whose pc-0 entry
  is the function's start line and column (so a stack trace through the trampoline frame
  names the function's start) followed by entries at pc > 0 that carry: the offset of the
  parameter list from `SRC_START`, the context bits (self binding of a named function
  expression; for arrows the kind of the function that owns `this`).

Size: about 48 + 4 per capture + info 32 + lines ~12 + names, against a compiled
template that has all that plus the code, handlers, constants and the real line table.

### 4. First call: the `lazy` instruction (a trampoline)

Every call path (`run_hot`'s `call_closure`, `call`, construct, native callers) pushes a
frame from the template and runs its code; a lazy template's frame has 0 locals and its
code is `lazy`. So no call path changes, and the hot loop gains nothing to check: `lazy`
is not a hot op, it declines to `step`. `lazy`:
1. `T' = T.IC`, or compile `T` (5.) and set `T.IC = T'`;
2. sets the closure's template (`ELEMENTS`, slot of `stack[bp]`) to `T'`, so `load_frame`
   and later calls see `T'`;
3. grows the stack for `T'`'s locals and operand stack (a RangeError on overflow),
   fills the locals with `undefined` and enters `T'` at pc 0, in the same frame (the
   arguments are padded to `PARAMS`, which is the same).

It changes nothing before the compile succeeded, so an out-of-memory compile collects
and retries like any allocating instruction (D1700); `HostOutOfMemory` stops the run.
Fuel: a charge proportional to the span's bytes (determinism needs only that it is a
function of the program). Other closures of the same template still enter `lazy` once and
take `T'` from `IC` (cheap); `fclosure` may later take `T'` directly (batch 4).

### 5. Compiling a lazy template

`compiler::compile_lazy(heap, t) -> Result<Ref, CompileError>`:
- reads the Script (source, options bits, file name, params) and the lazy record;
- copies the bytes `[SRC_START, SRC_END)` out of the arena (the parser borrows the heap
  mutably, and the arena may move) and parses them with positions relative to
  `SRC_START`: `finish` adds the base to every template's `SRC_START`/`SRC_END`, and the
  line table adds the base line and column (column only on the first line). Only the
  span is touched, so a function's first call costs O(its size), not O(script);
- sets up a synthetic outer function (id 0) whose body scope declares one binding per
  capture name, of kind Const / FuncName / Let / Var from the two attribute bits, and
  `this_var`, `home_var`, ... for the pseudo names; for arrows it takes the owner's kind,
  so `own_this_kind`, `declare_pseudo` and `super` checks answer as in the full parse. A
  dummy class context accepts private names (`#x` resolves through the captures). `await`
  and `yield` contexts are the permissive ones: valid code parses the same;
- positions the lexer at the parameter list and calls the existing routine for the
  kind (`parse_function_rest` with or without the self binding,
  `parse_method_function_dp`, `parse_arrow`);
- pre-seeds the function's capture slots with the outer bindings in the template's order,
  runs `finish` for ids >= 1 only, sets the root's name and capture descriptors from `T`
  and returns it. A capture count that differs from `T`'s is an internal error.

Nothing user-visible moves from load to the first call: the only errors `finish` reports
are "too many local variables" (checked for every function at load already, storage is
assigned for all) and too many captures (the lazy template has them too).

### 6. Code identity: eager and lazy compile the same bytes

Suspension maps (D2563) describe the code that runs, and they come from another compile
of the same source (the dumper's scratch compile, `compile_with_maps`). So a function's
code must not depend on whether it was compiled with its unit or at its first call. What
is unit-wide today: the shared constant pool (D2446), whose indexes (and `push_const8`
versus the wide form) depend on what else the unit holds. So pools follow the lazy
structure in every compile: the top level's pool, a pool for every lazy-eligible function
shared with its eager descendants, and the unit's shared pool for the rest. An eager
compile ("compile everything", used by maps, `djs disasm`, the compiler's tests) gives an
eligible function the same pool it gets when compiled lazily as a root. The IIFE and size
rules being local (2.) is what makes eligibility itself identical. Atom ids differ across
heaps already and are fixed-width immediates.

A unit test checks it over the corpus and the dump tests: every function's template
(code, constants, handlers, line table, names, flags) from an eager compile equals the
one from compiling lazily and forcing it.

### 7. Everything else

- **Generators, async**: lazy like others; flags come from the template, the first
  instruction of `T'` is `generator_start` / `async_start` as now.
- **Classes**: methods, getters and setters lazy; constructors, field initializers and
  static blocks eager (2.). Private names and the home object are captures by name.
- **eval / with**: there is no `with` and no direct eval, so the capture lists are
  static; that is what makes this design possible. Indirect eval code and its functions
  can be lazy (its Script keeps its source).
- **TypeScript**: the re-parse uses the Script's options bits; erasure works on the
  span. Const enum folding and namespace members pin (2.).
- **Source text**: the Script's source is a JS string (Latin-1 or UTF-16) with UTF-8 byte
  offsets in the templates. Copying a span needs either the span's code-unit offset in
  the lazy record (4 more bytes, computed in one pass at the unit's end) or, simpler, the
  Script keeping its source as UTF-8 bytes (a `Bytes` cell). The second also makes
  `Function.prototype.toString` O(span) instead of converting the whole source per call
  (`source_slice` does that today). Proposed: UTF-8 in the Script (a format change,
  COMPAT_VERSION; touches toString and the dumper's `text_of(Script::SOURCE)` readers).
- **Images**: an image must not depend on the build's features or need compiles after a
  restore. `Vm::seal` and the image writers first compile every lazy template; images
  never hold one. pi's sealed prelude image is therefore fully compiled, as now.
- **GC**: lazy templates are ordinary cells; `IC` keeps `T'` alive while `T` lives
  (`T` stays referenced from the parent's children). Allocation during the compile never
  collects; collection points stay a function of the program.
- **Determinism and tests**: lazy changes allocation sizes and fuel totals, so tests that
  pin heap sizes, collection counts or fuel will need new numbers.
- **The dumper** reads templates of closures that may never have run:
  `global_refs` and `code_globals` scan their code and their children's for globals
  (dump.rs), the harvest reads locals and names. A lazy template has no such code. The
  dumper gets the eager compile of the function from its scratch heap (it already compiles
  scripts there for maps) and scans that. Capture names already work.
- **Suspension maps and compile caches**: `compile_with_maps`/`suspension_maps` stay
  eager, which by 6. gives maps valid for lazily compiled code. A plain dump's load
  (`Vm::compile` keeps its maps in `dump_cache`) stays eager until batch 3.
- **The dump's check** already skips code generation (D2889, `compiler::free_names`); it
  is not affected.

## Size

Code that has to exist: eligibility and the pool split in `finish` (~0.5 KB wasm), lazy
template writing (~0.3 KB), `compile_lazy` with the synthetic outer and the base offsets
(~1.5 KB), the `lazy` instruction (~0.4 KB), forcing at seal (~0.2 KB): about 3 KB of
wasm, ~1.2-1.5 KB gzip. The ceilings have ~320 B (main) and ~110 B (slim).

Proposal: a cargo feature `lazy` in djs-core, on by default (native, CLI, tests) and
turned on by djs-wasm's `dump` feature, **off in the shipped main and slim modules**. The
measurements say why: codemode scripts gain ~10% of a ~40 µs compile at load and give
some of it back at the calls; the wins are in dumps (the dump module), large scripts and
the dump runtime. The `lazy` opcode stays in the table in every build (one opcode table,
one image format); without the feature its handler is an internal error, and images
never contain lazy templates (7.), so no build meets one it cannot run. If the owner wants
it in main later, it is a recorded size trade.

## Plan (batched merges)

1. **Core, opt-in.** Feature `lazy`; `CompileOptions::lazy` (default false, so
   `compiler::compile`, tests, disasm stay eager); eligibility, lazy templates, the
   pool split (only with the feature), `compile_lazy`, the `lazy` op, forcing at seal and
   image writing; the identity test; e2e/test262/difftest runs with lazy on (a `--lazy`
   flag; the coordinator decides which gate leg runs lazy: I suggest test262's active
   group and difftest's stress pass lazy, the plain difftest eager, so the gate does not
   grow). Guide: compiler.md gets a section on lazy compilation.
2. **On by default in lazy builds.** The dumper reads code through scratch compiles;
   Script source as UTF-8 (or unit offsets); `Vm::compile` compiles lazily; new numbers
   for pinned tests.
3. **Dump loads (with pjr, pjs).** A plain dump's load compiles lazily; maps for the top
   unit at load, and for other functions on demand at the re-dump (only those with
   activations need them). Measure corpus load and re-dump with scripts/dump-costs.
4. **Tuning.** Small-function threshold from the harvest corpus (count lazy compiles per
   run), `fclosure` taking `T'`, phase-1 emission off inside lazy subtrees.

## Risks

- Correctness of the re-parse context (5.): any parser decision that reads enclosing
  functions' state. Audited: `own_this_function`/`own_this_kind`, `declare_pseudo`,
  `await_reserved`, `completion_local`, `emit_bind_this`/`ctor_param_props` (constructor
  arrows stay eager), private names, namespace members and const enums (pinned). The
  identity test over the corpus and test262 in lazy mode are the net.
- Double parsing of called functions, multiplied by nesting depth for code like callback
  pyramids (each level parsed once per enclosing lazy level). Bounded by depth x size;
  IIFEs are eager.
- Pool split changes the constant indexes of every user function in lazy builds; the
  shared pool of D2446 shrinks for user code (not for the prelude).
- The dumper (batch 2) is the largest consumer of template internals; lazy stays off by
  default until it is adapted.
- Size: off in main/slim; a later wish for main is a size trade.
