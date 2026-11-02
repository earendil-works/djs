# What djs can learn from pico

Issue 0152, research only: no engine change. Source: pico's documentation site as one
HTML file (work/scratch/0152 of the primary checkout, with a text extraction), read in
full: the tour, the language reference, and the generated appendix (opcodes, cell
layouts, image format, wasm exports). pico's sections are cited as *pico: Tour ›
Chapter*. djs numbers are from main at 7d684dbb (0151's L6 is on its branch) unless
stated. The benchmark checks in section 3 ran on an arm64 Mac, node 25.8.1, with a fresh
`wasm`-profile build of djs-wasm and quickjs-wasi 3.6.2 (pi's engine), with the scripts in
work/scratch/0152/w.

## In short

- **pico and djs share the core design.** One arena of offsets, no pointers. Each VM
  starts from a base image made at build time. The interpreter never recurses. Callbacks
  are never called from native code, so the stdlib is split into leaf natives and VM code.
  Results are applied in a fixed order, and fuel decides where a run stops. pico differs
  where it controls the language: JSON values without identity, no cycles, reference
  counting instead of a collector, static types that pick typed instructions, and a JIT
  that needs no speculation. Those parts don't carry over to JavaScript.
- **What does transfer, ranked:**
  1. Fix three slow spots that pico's benchmarks expose; one of them is 26x slower than
     quickjs-wasi.
  2. Four concrete shapes for 0151's base shrink:
     - one cell per function;
     - call sites that skip forwarding wrappers;
     - 8-bit operands with an extension word;
     - static calls.
  3. Store tool results once, as a buffer that slices point into, and let snapshots leave
     it out.
  4. Delta images that need no codec.
  5. Seeded hashing against crafted tool data.
  6. Check image compatibility by what the image depends on, not by the build.
  7. Charge fuel only at safepoints.
  8. Write large results straight into place.
- **pico's numbers for djs are partly stale and partly right.** The image row predates
  session images: it gives 336 KB, while a new sandbox snapshots to 11.7 KB, or 1.2 KB
  with zstd (measured). On this machine the compute cases put djs at 0.9–1.5x quickjs-wasi.
  pico's table has 0.8–4x. The difference is in the scripts, not in djs's general speed.
  Rewriting the cases showed which idioms are slow:
  - `[...Array(n).keys()]`: 26x quickjs-wasi;
  - `Array.from({length: n}, f)`: 2.9x;
  - `map`/`reduce` on arrays of 0–2 elements: a recursive tree walk is 2x.

  The peak-memory gap on big data (2–3x pico) is real. It comes from collector headroom
  and copies of tool text.

## 1. Area by area

### Values, cells and the arena

*pico: Tour › The arena and the value encoding; Cells, strings and lists; Appendix ›
Cell kinds and layouts.*

- **Values.** Both NaN-box 64-bit values with 32-bit offsets. pico's `num` excludes NaN
  and the infinities, so integers take 48 bits inline. djs must keep NaN and has int32
  fast paths ([values](guide/values.md)). Nothing to adopt.
- **Allocator.** pico's arena is a sequence of 64 KiB pages: one size class per page (19
  classes), free lists, and a page table. Cells never move, so C code may hold pointers
  while it runs. The cost is a floor: a page per size class in use. pico's own table
  shows a 1.4 MB arena for `return 1`. djs's fresh VM is a 322 KB arena, and its
  compacting collector packs cells in allocation order ([the garbage
  collector](guide/gc.md)). **For per-VM memory, djs's layout is better.** pico's
  non-moving pages pay off for deltas and for large strings: a non-moving large-object
  space is ARCH.md's open item.
- **Cells.** pico's header is 8 bytes (refcount, kind, flags, aux). A compiled function
  is one `Func` cell: a 48-byte header (counts, name, file, and refs to consts, field
  slots and the line table) followed by its code words inline. In djs a builtin is
  several cells: a function object (32 B plus captures), a template (56 B), a bytecode
  cell, a pool cell and a capture descriptor (0151's plan). **This is lever L3 of 0151;
  see 2.2.**
- **Objects.** pico stores an object as entries in insertion order, 16 B each (atom and
  value). Lookups scan linearly, and each `GETF`/`SETF` instruction carries an inline
  cache with the last index. djs's shapes cost 8 B per value plus a shared shape
  ([objects and shapes](guide/heap.md)): better for many objects of the same shape, the
  common tool-result case. Nothing to adopt.
- **Strings.** pico stores UTF-8 with a cached code-point length and a code-point cursor.
  djs stores one-byte or two-byte strings, plus ropes and append buffers, because JS
  needs UTF-16 code units. pico's **slices and packed lists** do transfer; see 2.3.
- **Atoms.** Atoms that pico makes from data are reference counted and leave the table
  when they die. djs's atom table only grows for the life of a VM. That is harmless for
  short codemode VMs, so no lesson. pico's atom set is probed with **SipHash-1-3 under a
  per-run key** taken from the host's seed, so it is deterministic and flood-resistant.
  See 2.5.

### Memory: reference counting

*pico: Tour › Values and reference counting; Benchmarks › Memory and modules.*

pico can count references because its language has no cycles. Values behave as copies,
and a closure that escapes captures values, not variables. Collection becomes
deterministic and incremental; an image never holds garbage; and peak memory tracks live
memory. A JS engine cannot do this: objects have identity, and cycles are everywhere,
starting with prototype↔constructor. The useful observations are about cost:

- **Peak versus live.** djs lets the heap grow to twice the live size before a collection
  (D1870, [the garbage collector](guide/gc.md)). On pico's large cases djs peaks 2–3x
  higher than pico. Examples: tree depth 22 at 1,713 MB against 579; map-filter on 10M
  at 514 against 175. Part of that is the factor 2, and part is object size. On this
  machine, a depth-20 tree of `{v, l, r}` peaks at 169 MiB, about 84 B per node with the
  headroom. A smaller growth factor above some heap size would trade collections for
  peak memory. That only matters for scripts with a heap in the hundreds of MB, which
  codemode rarely has (0130), so it ranks low.
- **pico pays for counting with every register write.** `cell_drop` takes 4–8% in its
  object-heavy cases, and only the JIT removes the count traffic, for typed registers.
  This confirms that djs's bump allocation without a write barrier is the cheaper
  steady state.

### Compiler and bytecode

*pico: Tour › The compiler; Appendix › Opcodes.*

- **pico is a register VM** with 83 opcodes. An instruction is a 32-bit word: an 8-bit
  opcode with A, B, C operands of 8 bits each, or A and a 16-bit Bx; some take extra
  words. A function has at most 255 registers. Static types pick **typed instructions**
  (`ADDI`, `ADDN`, `CONCAT`) and **fused forms**:
  - compare and branch, also against an immediate (`JLT`, `JGEK`);
  - add an immediate (`ADDK`);
  - a counted loop back-edge (`FORI`);
  - **static calls** with the callee in the constant pool (`CALLF`);
  - **direct native calls** (`CALLNAT`).

  Calling a stdlib overload whose body only forwards to a native compiles to `CALLNAT`:
  the wrapper is never entered.
- **djs is a stack VM** with variable-length instructions, emitted by a single pass that
  builds no AST (G3, [the compiler](guide/compiler.md)). The typed instructions need
  pico's sound types; TypeScript's annotations are erased and cannot be trusted, so that
  part does not transfer. The **encoding** lessons do transfer to privileged code (2.2):
  - pico shows that a whole VM lives with 8-bit operands plus an escape;
  - the fusions it found worth having (compare with an immediate and branch, static
    call, direct native call) match what 0151 found in the prelude.
- **Diagnostics for models.** pico's parser recovers at statement boundaries and reports
  every syntax error in one compile. Most errors carry a hint that names the fix, often
  the pico form of a JavaScript habit. djs matches QuickJS's messages verbatim (D1100)
  and stops at the first SyntaxError, as QuickJS does. Changing that is out of scope.
  The part that transfers belongs to pi's adapter: check tool arguments against the
  tool's `inputSchema` before the call leaves, with the path of the first mismatch and
  a "did you mean `project`?" hint. pico does this in the compiler; a JS host can do it
  when the call is made.

### Interpreter, suspension, fuel

*pico: Tour › The interpreter and fibers; Suspension and fuel.*

- **The same model.** pico's loop never recurses, keeps frames as 16-byte records in the
  arena, and follows a leaf rule: natives never call VM code. djs has the same leaf rule
  ([builtins](guide/builtins.md)). pico gets away with 85 natives because its values
  have no getters, `valueOf` or proxies. In JS, almost any builtin that takes an object
  can run user code: 651 of djs's 874 builtins are reentrant (0151's classification).
  djs's equivalent of pico's natives is `@fast` leaves that decline when anything is
  observable, with bytecode or `@native` state machines behind them.
- **Fibers versus promises.** `pmap` and `par` give pico structured concurrency with
  cancellation: when one element fails, the other calls are reported to the host as
  cancelled (`takeCancelled`). JS promises have no cancellation; that stays the host's
  business (an `AbortSignal` in pi's tool wrappers).
- **Fuel.** pico charges 1 unit per call-like instruction and per taken backward jump,
  plus the work natives report. Its fast path is one decrement and compare at those
  points; every 1024 safepoints a slow path checks memory and stop requests.
  Straight-line code is bounded by the size of the code. djs checks fuel at **every
  instruction** and subtracts a per-op cost from a table (`run_hot`, interp/run/hot.rs).
  That gives "stop between any two instructions", which djs's stress modes use (suspend
  every 97). Fuel is not part of djs's semantics (D1633), and any counting model gives
  determinism. See 2.7.

### Host calls and determinism

*pico: Tour › Host calls and determinism; The host interface; Appendix › Wasm exports.*

- **Equivalent:**
  - calls reported as `{id, fn, args}`;
  - answers applied one at a time, in the host's numbering, each with a clock value;
  - a seeded random generator;
  - no other source of time.

  djs does the same with its inbox (D1670) and host-set clock and seed
  ([determinism](guide/determinism.md)). pico makes the numbering explicit
  (`deliver(seq, id, outcome, now)`); djs applies results in delivery order. Both work.
- **Different:**
  - **Results land in a result buffer** (`pico_reserve_result`): the host writes the
    payload straight into the cell that the value's strings will point into, so a result
    is copied once. djs writes into a staging buffer, then transcodes into heap strings
    ([wire format](wire.md)). See 2.3 and 2.8.
  - **Hash keys** are seeded per run (2.5).
- **The same ABI style:** no imports, scratch in, out buffer out, numbered statuses, a
  panic message on trap. pico has 43 exports plus 23 for the JIT; djs has about 45.

### Images and snapshots

*pico: Tour › Images and snapshots; Appendix › Image format.*

- **How pico gets images of a few KB:**
  1. **Delta against a shared base.** The base is the std base embedded in pico.wasm, or
     a module-set base made by the host.
  2. **Scrubbing.** All free memory below the top is zeroed before an image is taken, so
     dead bytes match the base or form zero runs.
  3. **A run table.** One word-at-a-time pass finds the stretches that differ from the
     base and stores each as (gap, length) LEB128 pairs plus its bytes. A 64-byte header
     carries the ISA version, the image format, the base id (a checksum of the base's
     own image) and a checksum.

  Measured by pico: 4.7 KB for a waiting script, taken in 0.38 ms and restored in
  0.44 ms. Because cells never move, a delta needs no collection.
- **djs's design differs and reaches about the same sizes.** A snapshot is a session image
  (D2066): the builtin prefix is left out, and prefix words that differ are stored as
  offset/value pairs. The container is compressed with zstd against a reference image
  (D2260). The collector keeps a reference's cells in place, so zstd finds them as one
  match ([snapshots](guide/snapshots.md)).
  - A new sandbox measures 11.7 KB uncompressed and 1.2 KB with zstd.
  - A pi session waiting on a tool is 2–8 KB with zstd.

  Without zstd (browsers, D2260's trade-off), the session region is stored whole.
  pico's run table is how to get small images without a codec; see 2.4.
- **External results.** In pico's external results mode, an image lists each live
  result buffer as (call id, offset, length, hash) instead of storing it. The host
  supplies the bytes again from its log on restore. Measured by pico on a 5 MB log split
  into lines: 5.44 MB embedded against 0.44 MB external. djs's equivalent today is a
  pi session holding a 1.5 MB log: 2.2 MB raw, 404 KB compressed. See 2.3.
- **Compatibility.** pico checks the ISA version, the image format and the base id, not
  a build hash. Opcodes are only appended, and any change to them bumps the ISA version
  by hand. Its images therefore survive a rebuild that changes C code but not the base,
  the opcodes or the formats. djs's build hash covers djs-core's sources (D1330), so any
  change refuses every image. See 2.6.
- **Compaction above the base** (`heap_compact_above`) slides only the cells above the
  base's extent, so the delta does not grow. This is djs's prefix (D2064) and old region
  (D2041) under another name.

### The JIT

*pico: Tour › The JIT; Benchmarks.*

- **The invariant.** Compiled code keeps values in wasm locals, but it performs the
  interpreter's reference count operations, frame pushes and allocations in the same
  order. Suspending is a flush, images contain no code, and the test oracle is "images
  byte-identical to the interpreter's after every fuel slice". The JIT emits one wasm
  module per hot function, which the host compiles and installs into the function table
  between runs.
- **What it buys:**
  - 5.2x on an int loop, 1.8–2.5x on map/filter, calls and closures;
  - 1.1x on objects and on fib from a cold start;
  - nothing on native-heavy work (logs, JSON).
  - **It costs on short scripts:** fib(25) takes 8.27 ms with the JIT "on" against
    5.40 ms interpreted, because installation costs more than it returns.

  pico ships it **off by default**. Its inference needs no speculation only because the
  instructions are already typed.
- **For djs:** JIT is a stated non-goal, and pico's numbers support that for codemode.
  Scripts are short and spend their time at the boundary and in natives. A JS JIT would
  need speculation and deoptimization on top. The idea worth keeping on file: a tier that
  leaves the arena identical to the interpreter's at every safepoint is compatible with
  suspend-anywhere, images and determinism, and has an exact test oracle.
- **V8 note** (pico: Benchmarks › A note on measuring wasm in V8). Under V8 14.6, the
  first native call in each new instance deoptimized pico's whole interpreter loop: V8
  speculatively inlines `call_indirect` targets, and a new instance's table entries are
  different targets. pico's fix calls natives through a switch on the native id. djs
  calls `@native` builtins and `@fast` entries through function-pointer tables, which are
  `call_indirect` in wasm, and the `Sandbox` makes an instance per sandbox. Measured on
  node 25.8, the runs with a fresh instance each are tight (min ≈ max), so djs is not
  affected today. **Re-check when pi moves to node 26 / V8 14.6.**

### Testing

*pico: Tour › How pico is tested.*

- **The same approach** as djs ([testing](guide/testing.md)):
  - oracles over hand-written expectations;
  - snapshot and restore after every N units must change nothing;
  - every documentation example runs in the docs check (djs's `js live` blocks).
  - benchmarks double as a differential test: every engine must return the same value.
- **Additions worth copying:**
  - **Suspend after every single fuel unit** (`--suspend 1`) on the golden scripts,
    besides 97. djs suspends every 97 in the gate, and every 50 in the guide's examples.
    A cheap gate addition is `--suspend-every 1` on a small fixed set, such as the
    guide's live examples.
  - **Model evaluations.** 20 tasks with fixtures and reference answers ask whether
    models write working pico from its description. Model mistakes feed back into the
    error messages. djs's harvest (2,782 real executions, 0149) answers the same
    question for JS after the fact. An eval would be pi's to run.

## 2. Lessons, ranked

Value is for djs's goals: codemode speed, snapshots, per-VM memory, module size. Effort
is agent time, a rough estimate.

| # | lesson | goal | value | effort | touches |
| --- | --- | --- | --- | --- | --- |
| 1 | fix the slow idioms pico's cases hit | speed | high | days each | interp/run/iter.rs, builtins/array.ts, `%EachDense` (0137) |
| 2 | shapes for 0151: one cell per function, skip forwarding wrappers, 8-bit operands, static calls | per-VM memory | high | inside 0151 | 0151 L2–L4 |
| 3 | tool results as one buffer with slices; snapshots leave it out | memory, snapshots, G1 | high for big results | weeks | heap/string.rs, wire, vm::session, js/src/snapshot.ts, pi adapter |
| 4 | run-table deltas against the reference, no codec | snapshots (browsers) | medium | days | vm::session (D2066), js/src/snapshot.ts |
| 5 | seeded hashing for the VM's own atoms and Map/Set | robustness | medium | days | heap/atom.rs, heap/collection.rs, heap/string.rs |
| 6 | image compatibility by tables and base, not sources | snapshots across builds | medium, more after 1.0 | days + a policy | D1320/D1330 build hash, containers |
| 7 | fuel at safepoints only | speed | unknown: measure first | 1–2 weeks | interp/run/hot.rs, step, stress modes |
| 8 | write large results straight into place | memory, G1 | low–medium | days | wire, djs-wasm ABI, js/src/sandbox.ts |
| – | not to adopt: reference counting, typed instructions, a JIT, page allocator, 48-bit ints | | | | |

### 2.1 Fix the slow idioms (speed)

pico's compute cases are pico scripts translated to JS; the sources are in pico's bench/
and not in the site. I rewrote them from their descriptions (section 3). Where a
rewrite put djs near quickjs-wasi but pico's table did not, the difference is the idiom.
Three stand out:

- **`[...Array(n).keys()]` and `Array.from(Array(n).keys())`: 562 and 595 ms for n = 1M,
  against 21.6 and 22.8 ms in quickjs-wasi (26x).** `Array(n)` is all holes. D1800's
  `iter_next` fast path (`iter_plan`, interp/run/iter.rs) declines on a hole for every
  iterator kind. A keys iterator never reads the element, though, so it could produce
  `i` whenever `i < length`. A one-line condition, plus the same check in
  `%IteratorFastReady` for spread and `Array.from`. This is the usual range idiom of
  model-written JS.
- **`Array.from({length: n}, f)`: 132 ms against 46 (2.9x).** A push loop is 66 against
  39. The array-like path with a mapping function is worth a look.
- **Callback builtins on tiny arrays.** A tree whose nodes keep `children` arrays, summed
  with `children.map(...).reduce(...)`, takes 794 ms against 408 (1.95x). That is pico's
  9b ratio (457/233). With plain `{v, l, r}` nodes and direct recursion, djs is at 0.94x.
  0137 recorded this cost: `%EachDense` made calls on empty or one-element arrays 1–4k
  instructions dearer. pico's tree case is the benchmark for that follow-up.

### 2.2 For 0151 (per-VM memory): the base shrink

Four things pico does that map onto 0151's levers (plan in
work/issues/0151-base-to-a-fraction.md):

- **L3, one cell per function.** pico's `Func` is the template and the code in one cell:
  a 48-byte fixed part with refs to consts, field slots and the line table, followed by
  `code[]` inline (pico: Appendix › Cell kinds and layouts, `Func`). For djs's builtins
  this means the bytecode inline in the template. That saves the bytecode cell's header
  and padding (0151 measured 13.6 KB of headers and padding for the code cells), and one
  reference per template. A privileged template could also drop the fields that pico
  doesn't have for its std either (0151's 56 → ~32 B).
- **L2, call sites that skip forwarding wrappers.** pico's checker sends a call to a
  stdlib overload whose body only forwards to a native straight to `CALLNAT`; the
  wrapper is never entered. JS needs the *visible* builtins to stay function objects,
  hence 0151's thin native objects. But privileged code can do what pico does for
  *internal* helpers:
  - a prelude helper whose body is `return %I(args)` (or `return F(args)`) can be
    compiled as `call_intrinsic` at every privileged call site;
  - a helper that nothing else references then disappears from the base: no object,
    template, code or name.

  0151 counts 223 internal helpers (62 KB) and 214 thin functions. The intersection is
  the candidate set. This works once L6's `get_builtin` table exists, since then every
  call site names its callee statically.
- **L4, operands.** pico runs a whole VM on 8-bit register, constant and argument
  operands with a 16-bit form and extension words (pico: Appendix › Opcodes). This
  supports 0151's narrow-operands-with-wide-forms plan: 7,749 of 7,749 locals and
  1,076 of 1,076 constants fit 8 bits. The fusions pico chose are candidates for
  privileged code:
  - compare with a small immediate and branch;
  - `ADDK`;
  - a static call naming its callee (0151's `call_builtin idx argc`: 1,580 sites of
    `get_cap; undefined; args; call`).
- **What not to copy:**
  - pico keeps line tables for its std (traces show `std.pico:115`); djs dropped them for
    the prelude (D1430), which is right.
  - pico's std is small because its values have no protocol. The JS-compatible version
    of "natives for everything that takes no callback" is 0151's L7 (134 complex leaf
    builtins to Rust, −16 KB per VM for about +12 KB gz). Going further is bounded by
    reentrancy: 526 builtins are complex and reentrant.

Also for 0151's context: pico's per-VM floor is larger than djs's. Its fixed memory is
2.9 MB against djs's 1.5 MB, made of a 1 MiB C stack and a page per size class.
pico's "images of a few KB" are deltas, not resident memory: its own table gives a
1.4 MB arena for `return 1`. 0151 targets djs's raw base, the resident memory per VM,
and on that measure pico's design is no model to follow.

### 2.3 Tool results: one buffer, slices, and external results

*pico: Tour › Cells, strings and lists (Slices and packed lists); Images and snapshots
(External results).*

pico stores each extern result in one buffer cell tagged with the call id and a content
hash:

- a text result is a slice of it;
- the long escape-free strings of a JSON result are slices of it;
- `lines()` and `split()` give packed lists of 8-byte (offset, length) pieces.

On a 5 MB log split into 54k lines, the VM holds the buffer, 0.43 MB of pieces and 5 KB
of state. An image in the external mode leaves the buffer out, and the host re-supplies
it from its log.

For djs, JS semantics allow the parts that matter:

- **A sliced string kind**: a one-byte (or two-byte) string that is `[off, off+len)` of
  a flat parent. This is V8's SlicedString. `slice`, `substring`, `split`, the strings of
  a regex match and the lines of a split over a big flat string would share the parent
  instead of copying.
  - Cost: every reader of string units must handle the new kind, or it must be flattened
    on demand the way ropes are.
  - The collector must keep the parent. That is also V8's retention risk: a small slice
    pins a big parent, so slice only above a minimum length and let the collector copy
    out short survivors.
  - Packed arrays have no JS equivalent (an Array of strings is observable element by
    element). The slices give most of the benefit anyway.
- **A result buffer** for a large string result: the host writes the payload into one
  cell tagged with the call id and a hash (2.8), and the value is a slice of it. Only
  one-byte payloads qualify, which is ASCII from the host's point of view: the common
  case for logs, files and JSON text. Two-byte payloads take today's path.
- **External results in snapshots.** The session image lists live result buffers as
  (call id, offset, length, hash) instead of storing them. pi already journals tool
  results for replay (D1730), so it can supply them on restore, and a missing or changed
  payload is refused by its hash.

What it would buy, from pico's numbers and djs's:
- pi's 1.5 MB-log session snapshot would drop from 404 KB compressed to the session's
  own state (a few KB to tens of KB).
- djs's peak memory on pico's log and pmap cases is 2.2–2.5x pico's: 41.4 against
  18.7 MB, and 26.8 against 15.5.

This is the largest design change here: weeks, a new string kind and a snapshot format
change. ARCH.md's unbuilt "host refs" (opaque pass-through values) is the neighbouring
idea, for data a script only forwards.

### 2.4 Delta images without a codec (snapshots)

djs's session image already diffs the prefix as offset/value pairs (D2066). Taking the
**whole** image as a run table against the reference (gap and length as LEB128 varints,
then the differing bytes, as pico's format) would give small snapshots with no zstd:

- the reference's cells stay in place under djs's collector, which is what makes zstd
  work today;
- what pico adds is scrubbing, and djs's compaction already leaves no garbage.

This helps where zstd is missing: browsers, the D2260 trade-off. It is also faster than
compressing, by pico's numbers: 0.38 ms to take a 4.7 KB image against djs's ~1.2 ms of
zstd on a waiting session. zstd can still run on top of the runs. Effort: days, in
vm::session and js/src/snapshot.ts; a container image kind.

### 2.5 Seeded hashing (robustness)

djs hashes strings with a fixed multiplicative hash (heap/string.rs) for the atom table
and for Map/Set (heap/collection.rs). Tool data is untrusted (web pages, issue
trackers), so JSON keys or Map keys built to collide could make a script quadratic.
pico keys its atom set and its natives' hash tables with SipHash-1-3 under a per-run key
taken from the host's seed, so runs stay deterministic. For djs:

- the base's atom index in the prefix keeps its fixed hash (its keys are known, and the
  prefix must be identical in every VM, D2064);
- a VM's own atom index (D2431's second level) and Map/Set use a key derived from the
  VM's random seed, which is image state already.

Cost: a few days, a slower hash on the VM's own atoms (measure), and string hash caches
that depend on the VM.

### 2.6 Image compatibility by what the image depends on (snapshots)

pico refuses an image only for a different ISA version, image format or base id. djs
refuses any other build (D1320/D1330: the build hash includes a hash of djs-core's
sources), so a fix in a leaf intrinsic invalidates every stored session. In
docs/goldilocks-snapshots.md section 4, the build hash breaks images in 70% of issues
(43% of the last 30).

A key made of what images actually depend on would let images survive Rust-only changes:
- the layout table;
- the opcode table;
- the intrinsic and native tables;
- the base prefix hash (already in the session format);
- a manual "heap semantics" version that reviewers bump when Rust code reads heap state
  differently, as pico bumps its ISA version.

During development the base changes in most issues, so the gain is small now. It grows
once the base stabilizes, and it costs days plus a review rule. It is the cheap middle
between today's images and 0142's by-meaning tier 2.

### 2.7 Fuel at safepoints only (speed: measure first)

djs's `run_hot` checks fuel at every instruction boundary and subtracts a table cost
per op. pico charges at calls and backward jumps only, which bounds the work between
checks just as well. Moving djs to that model would drop one load, one subtraction and
one compare-and-branch from every simple op. But:
- the stress modes and the fuel-slicing determinism tests would count safepoints
  instead of instructions;
- "stop between any two instructions" becomes "at any safepoint". pico shows that is
  enough for snapshots and determinism.

The gain is not known: measure it first with an experiment build (no fuel in straight-line
ops) and wasm instruction counts on all workloads. Only then decide.

### 2.8 Write large results straight into place (memory, G1)

`pico_reserve_result(call, len)` returns a pointer inside the result buffer, and the host
writes the payload there: one copy, and no scratch high-water mark. djs encodes into a
staging buffer and then transcodes into a heap string. For a big string result that is
ASCII (`encodeInto` reports as many bytes written as characters read), the host could
write straight into a reserved one-byte string cell. That saves a copy of MBs and the
staging buffer's growth. It is small on its own (days) and natural with 2.3.

### Not to adopt, and why

- **Reference counting, value semantics, escape rules for closures:** pico's language
  rules, which JS cannot have.
- **Typed instructions and a JIT without speculation:** need sound static types. A JS JIT
  is a non-goal, and pico's own numbers show little gain for short, boundary-bound
  scripts.
- **Page-segregated allocator:** a higher per-VM floor. djs's compacting arena is the
  better fit for many small VMs; a large-object space is the open item.
- **48-bit inline integers, no NaN:** JS Number semantics.
- **Rejecting JavaScript habits at compile time:** djs runs JavaScript. Hints belong in
  pi's adapter (schema checks of tool arguments).

## 3. pico's benchmarks against djs

*pico: Tour › Benchmarks (generated from pico's bench/RESULTS.md).*

**What they measure.** Wall time per script in node on an i9-13900 (x86-64 Linux, Node
26.1, V8 14.6). Medians of 3–31 runs after warm-up, module compilation excluded, every
engine checked to return the same value. Engines:
- pico in three modes;
- quickjs-wasi 3.6.2 driven like pi's codemode;
- "djs: the shipped wasm build, with its Sandbox API and async host functions";
- V8 directly, as a ceiling.

**Which djs.** 1.01 MB raw, 446.9 KB gzip. That fits main around 0145–0150: this
checkout's build is 1,064,790 B, and the gzip ceiling is 459,164 B. The time and memory
rows are therefore current enough. **The image row is not.** It reports a fresh djs
image of 336.2 KB, a raw arena from before session images (D2066) and the atom split
(D2431). Today, measured with `Sandbox.bytes()`:

| new sandbox | size | snapshot | restore |
| --- | ---: | ---: | ---: |
| uncompressed session image | 11.7 KB | 2.3 ms | 1.3 ms |
| zstd against the base | 1.2 KB | 2.9 ms | 0.5 ms |

pico's fresh image is 9.4 KB. The table also compares pico's delta with djs's raw arena
and with general-purpose zstd, not with djs's containers against a reference.

**Their ratios and ours.** Ours are on an arm64 Mac, node 25.8, with djs-wasm at
7d684dbb and quickjs-wasi through the perf harness (`scripts/perf/wasm-run.ts`,
`qjs-wasm-run.mjs`, fuel slice 10,000, median of 5). The scripts are rewritten from pico's
descriptions (work/scratch/0152/w).

| case | pico's table: djs / QuickJS | ours: djs / quickjs-wasi (ms) |
| --- | ---: | --- |
| 6a build 1 MB with `+=` | 1.40 | 1.32 (10.8 / 8.2) |
| 7a arithmetic loop 3M | 0.80 | 0.90 (60.0 / 66.7) |
| 7b map+filter+sum 1M | 3.06 | 1.28 push-built, 1.82 `Array.from`, 5.67 `[...keys()]` |
| 7c object churn 200k | 1.82 | 1.03 (107.9 / 104.6) |
| 9a fib(25) | 1.35 | 1.21 (7.5 / 6.2) |
| 9b tree depth 16, walk 5x | 1.96 | 0.94 `{v,l,r}`, 0.94 class, 1.95 children + map/reduce |
| 9c small calls | 1.07 | 0.94 (79.0 / 83.6) |
| 9d stored closures | 1.30 | 1.51 (63.0 / 41.6) |
| 10b props, script-built | 1.31 | 1.14 (27.6 / 24.2) |
| 11b map+filter 10M | 4.01 | 1.36 push-built (1,625 / 1,194; djs peak 395 MiB) |
| 11d tree | 1.26 | depth 20: 0.63 (523 / 830; djs peak 169 MiB) |

**What they reveal:**

- **Where pico's djs/QuickJS ratio exceeds ours, the idiom is the reason** (2.1). The
  likely translations of pico's `range` and `xs.map(...).sum()` are exactly djs's slow
  spots. The arm64/x86-64 difference may add to it: V8's register pressure on x86-64 hits
  a large interpreter loop harder, and djs is tuned on arm64. pi's servers may run on
  x86-64, so **a wasm benchmark run on x86-64** would be worth having.
- **The boundary cases** (records, pass-through, props-tool, pmap-200) put djs at 1.5–2.3x
  pico's interpreter and at 0.5–1.0x QuickJS. pico's breakdown shows those cases are dominated by JSON
  encoding and copying on both sides of the boundary. 2.3 and 2.8 are the djs levers.
- **Startup:** djs is the fastest of the three with tools: 0.74 ms against pico's 1.88
  (0.97 from its cached module image) and QuickJS's 1.17. Bare startup is 0.60 against
  0.52.
- **Memory:** djs's peaks are higher on big data (2.3); pi's sessions are small.
- **pico versus djs on compute** is mostly pico's types. On the 3M loop, pico's
  interpreter is 38 ms against djs's 256 on their machine: a register VM with typed,
  fused instructions. With the JIT it is 19 ms. That gap is not reachable for a JS
  interpreter without type feedback.

## 4. Follow-ups (for the coordinator)

1. **Keys iterators over holes**: `iter_plan` should produce `i` for a keys-kind Array
   Iterator whenever `i < length`, holes included. Also look at `Array.from({length},
   f)`. Small; measured 26x and 2.9x against quickjs-wasi.
2. **Callback builtins on 0–2-element arrays** (0137's follow-up): use pico's 9b shape
   (children arrays with `map`/`reduce`) as its benchmark; 1.95x today.
3. **Pass to 0151** (bloat): sections 2.2 and 4 are for 0151's base shrink:
   - one cell per function;
   - compile thin internal helpers to direct intrinsic calls and drop them;
   - 8-bit operands with an escape;
   - static calls.
4. Design issue: **sliced strings, result buffers and external results in snapshots**
   (2.3, 2.8).
5. Small issue: **run-table session images** for codec-free hosts (2.4).
6. Small issue: **seeded hashing** for the VM's own atoms and Map/Set (2.5).
7. Owner decision: **image compatibility by tables and base** instead of the source hash
   (2.6).
8. Experiment: **fuel at safepoints only** (2.7), measure before deciding.
9. Watch item: re-run the fresh-instance benchmark under node 26 / V8 14.6 (the
   `call_indirect` deopt pico hit).
10. Benchmarks on x86-64 wasm, beside the arm64 numbers.
