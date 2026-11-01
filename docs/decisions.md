# Decisions

An append-only log of implementation decisions. Newest at the bottom. To revise a
decision, add a new entry that supersedes it and say so.

Format:

```markdown
## DNNN: Title
date: YYYY-MM-DD · by: <agent/coordinator> · issue: NNNN
Decision, with the reason in one or two sentences. Alternatives considered, if relevant.
```

## D001: Work tracking in files, worktrees in `.worktrees/`
date: 2026-11-01 · by: coordinator · issue: -
Issues live in `work/issues/`, the board in `work/BOARD.md`. Sub-agents work in git
worktrees under `.worktrees/` (gitignored) and merge with `scripts/merge-to-main.sh`.
Scratch space is `work/scratch/`, external checkouts go in `vendor/`, and both are
gitignored.

## D002: dquickts is read-only
date: 2026-11-01 · by: coordinator · issue: -
`~/Development/dquickts` is only used through its existing `qjs` and `run-test262`
binaries and as reference reading. We never build or modify it.

## D003: test262 pinned to dquickts' commit
date: 2026-11-01 · by: coordinator · issue: -
`7ab7fafa0003f73fc85c1b95d88094d33f7eb8bd`, so oracle results are directly comparable.

## D004: Script goal in addition to the async body
date: 2026-11-01 · by: coordinator · issue: -
The compiler supports a Script mode (global var/function bindings on the global object, a
global lexical scope, `this` = global) next to the codemode async-function-body mode.
test262 needs it, and it is cheap. Neither mode is sloppy: both are always strict.

## D005: Toolchain
date: 2026-11-01 · by: coordinator · issue: -
Stable Rust (1.99 at the time of writing), edition 2024, a Cargo workspace with
`crates/*`. `djs-core` must build for `wasm32-unknown-unknown` in the gate.

## D1030: Lexer shape: context-free, span-only tokens, keyword ids
date: 2026-11-01 · by: lexer · issue: 0003
The lexer (`djs_core::lexer`) is a `Copy` struct of (source slice, `u32` position) and
does not allocate. A token is 12 bytes: kind, keyword id (`Kw`), flags and a byte span;
values are derived from spans on demand (`ident_value`, `cook_string`, `cook_template`,
`template_raw`, `number_literal`, `regex_parts`). Scanning validates everything, so
cooking cannot fail, except template escapes that are valid only in tagged templates.
Reserved words (including `enum`, which ECMAScript always reserves) have their own token
kinds; contextual keywords, the strict-only reserved words (`let`, `static`, `yield`,
`implements`, `package`, ...) and the TS keywords are `Ident` with a `Kw` id. An escaped
keyword is always `Ident` with `HAS_ESCAPE` and its `Kw`, so the parser can reject it
where a keyword or binding is required. The parser drives the context-dependent parts by
rescanning: `rescan_as_regex` (`/`, `/=`), `rescan_template_continuation` (`}`) and
`rescan_greater` (`>` is always lexed alone, tsc-style). Invalid UTF-8 anywhere,
comments included, is a SyntaxError ("invalid UTF-8 sequence"); QuickJS skips it in
comments. Error messages use QuickJS wording where QuickJS has one.

## D1031: Unicode tables from UCD 17.0.0, packed ranges
date: 2026-11-01 · by: lexer · issue: 0003
`scripts/gen-unicode.py` downloads `DerivedCoreProperties.txt` and
`extracted/DerivedGeneralCategory.txt` of Unicode 17.0.0 (the version dquickts'
`unicode_download.sh` uses, so it matches the pinned test262) into `vendor/unicode/` of
the primary checkout, and writes the checked-in `crates/djs-core/src/unicode/tables.rs`
(`--check` verifies it is current). Tables cover only non-ASCII code points as sorted
`u32` words `start << 11 | (len - 1)`, searched by binary search: ID_Start (737 words)
plus ID_Continue-minus-ID_Start (398 words) and Zs, about 4.5 KB in total. A bitmap trie
would be faster but larger, and non-ASCII identifiers are rare in practice.

## D1032: Line table semantics
date: 2026-11-01 · by: lexer · issue: 0003
`lexer::LineTable` (built on demand from the source) breaks lines at every ECMAScript
line terminator: LF, CR, CRLF, U+2028 and U+2029. QuickJS counts only LF; the
difference shows only for lone CRs and LS/PS, which are rare. Lines and columns are
1-based, and columns count code points, like QuickJS (V8 counts UTF-16 units).

## D1050: Value encoding
date: 2026-11-01 · by: heap · issue: 0005
`Value` is a NaN-boxed `u64`. Upper 16 bits `0xFFF8` mean tagged: the high word is
`0xFFF8_0000 | tag` and the low word is the payload. Everything else is an `f64`, with every
NaN canonicalized to `0x7FF8_0000_0000_0000`, so numbers never collide with tags. Tags:
1 undefined, 2 null, 3 bool, 4 int32, 5 hole, 6 uninitialized (TDZ), and the reference
tags `>= 8`: 8 string, 9 object, 10 symbol, 11 bigint (reserved), 12 host ref (reserved,
pointing to a host-ref *cell*). A reference payload is a nonzero `u32` arena offset. All-zero
bits are `+0`, which is not a reference, so zeroed memory is trace-safe. `==` on `Value`
is bit identity; JS SameValue is `Heap::same_value`. Two words make the JS-side decoder
trivial (`hi >>> 16 === 0xFFF8`).

## D1051: Arena and cell header
date: 2026-11-01 · by: heap · issue: 0005
One little-endian `Vec<u8>` arena, `u32` offsets, every cell 8-aligned. The 8-byte header
is kind (`u8`, 0 invalid), GC bits (`u8`, 0 between collections), per-kind flags (`u16`)
and total size (`u32`, multiple of 8). Cells are back to back, so the heap is walkable. The
`Roots` cell sits at offset 0, which is why 0 can mean null; the copying GC copies it first
and Cheney-scans from 0. Allocation bumps, zero-fills, may fail with `OutOfMemory` against a
configurable cap (also on host allocation failure), and never collects. Access is
bounds-checked slice indexing, no `unsafe`; bulk copies take one slice
(`Arena::split_mut` to copy from older cells into a fresh one).

## D1052: Layout description macro
date: 2026-11-01 · by: heap · issue: 0005
All cell kinds are declared once in `heap/cells.rs` with `cells!` (and `records!` for tail
element structs). The declaration has explicit byte offsets, which keeps the layout
reviewable and matches what the JS reader needs. The macro generates typed `Field<T>` and `Tail<T>` handles
(`arena.get(cell, Str8::LEN)`), a const `CellLayout` per kind, the `LAYOUTS` table, and
compile-time checks (after the header, naturally aligned, no overlap). Field types `Value`
and `Ref` are traced; that is the whole trace description. `LAYOUT_HASH` (FNV over the table
and the well-known atom list) is stored in `Roots::VERSION` and is part of the snapshot
build hash. The JS-side reader is to be generated from `LAYOUTS`.

## D1053: String representation
date: 2026-11-01 · by: heap · issue: 0005
Three cell kinds: `Str8` (Latin-1), `Str16` (UTF-16LE), `Rope`. Constructors narrow, so a
`Str16` always has a unit above 0xFF. Equal strings therefore have the same width, and
equal flat strings have equal bytes. Concatenations shorter than 32 units are copied; longer
ones make a rope whose `ROPE_TWO_BYTE` flag is the OR of its children. Flattening writes a
new flat cell and redirects the rope (`LEFT` = flat, `RIGHT` = null, `ROPE_FLATTENED`).
Rope walks are iterative. Max length is 2^30-1 (as QuickJS). The hash is FxHash-style over
code units with a murmur3 finalizer, so it is representation independent. It is cached in
the cell, and 0 means not computed.

## D1054: Atom representation; symbols are atoms
date: 2026-11-01 · by: heap · issue: 0005
`Atom` is a `u32`: bit 31 set means an integer index atom (0..2^31-1, computed, never
interned); otherwise bit 30 distinguishes symbol atoms from string atoms, and the low 30 bits
index the heap atom table (`AtomTable`: entries `ValueArray` + open-addressing `U32Array`
index). Atom 0 is null. Ids are indexes, not offsets, so they are stable across a moving GC
and can live in bytecode. Every symbol gets an atom table entry, so every property key is
just an `Atom` and key comparison is a `u32` compare. Larger array indexes (2^31..2^32-2)
are string atoms that `atom_array_index` still recognizes. Well-known atoms and symbols have
fixed ids generated from a list in `heap/atom.rs`. Atoms (and so symbols) are never freed
for now; atom GC is a follow-up.

## D1055: Object storage
date: 2026-11-01 · by: heap · issue: 0005
An `Object` cell has class, proto, property table ref, elements ref and a tail of internal
slots. `PropTable` entries (24 bytes: key, flags, value-or-getter, setter) are kept in
insertion order. Up to 8 entries are searched linearly (compacted on delete); above that a
hash index of entry numbers is built, with tombstones, and compaction happens on growth.
Dense elements (arrays only for now) are a `ValueArray`. A dense slot always means
`{writable, enumerable, configurable}` data, or a hole. An array-index key lives in exactly one place. Index keys
in the table set `OBJ_INDEXED_PROPS`, which exotic classes must also set, so the element fast
paths can trust objects without it. A write may extend the dense part by at most 64 holes;
writes further out go to the table. An array's `length` is internal slot 0 (a Number) and is
synthesized as the own `length` property. `OBJ_LENGTH_READONLY` is a header flag.

## D1056: Heap images are verified on load
date: 2026-11-01 · by: heap · issue: 0005
`Heap::from_bytes` checks the magic and `LAYOUT_HASH` and then runs `Heap::verify`: the
cell structure, every traced slot (a valid encoding pointing at a cell start of a fitting
kind), kind invariants (counts fit, ref fields point to the right kinds, rope lengths
strictly decrease towards the leaves) and acyclic prototype chains. `verify` never panics.
After it passes, heap code may still panic on semantically odd data, but cannot loop forever:
hash probes are bounded, and ropes and prototype chains are known to be acyclic.

## D1020: test262 runner execution model
date: 2026-11-01 · by: infra · issue: 0002
`djs-test262` judges every engine with the same code (`engine::judge`); an engine only
reports what happened (completed / harness error / parse error / runtime error / timeout /
oom / crash, plus printed lines). Harness files and the test are evaluated as separate
Scripts in one realm, in the order assert.js, sta.js, doneprintHandle.js (async), includes.
A negative test passes when the phase matches and the thrown value's `constructor.name`
equals `type` (INTERPRETING.md wording). An async test passes when it completes and prints
`Test262:AsyncTestComplete` exactly once and no `Test262:AsyncTestFailure`. The job queue
is drained for every test, but unhandled rejections do not fail non-async tests. `strict`
is set on the job for everything except `raw` (and explicitly selected `noStrict`) tests;
djs ignores it (always strict), qjs prepends `"use strict";` to the harness, the test and
`$262.evalScript` sources so the oracle runs djs' model.

## D1021: qjs engine details
date: 2026-11-01 · by: infra · issue: 0002
One `qjs -m -e <driver> <spec>` process per test. The driver compiles the test with
`std.evalScript(src, {compile_only: true})` and runs it separately, so parse errors are
told apart from runtime errors exactly. `$262.evalScript` uses `std.evalScript` (a real
Script in the global scope, with global lexical declarations) instead of indirect eval,
which the issue suggested but which has different var/let semantics.
`$262.detachArrayBuffer` uses `ArrayBuffer.prototype.transfer` and is a no-op on an
already detached buffer. qjs runs without `--experimental`, so there is no Intl (as in
djs); dquickts' `run-test262` is built with Intl.

## D1022: test262 grouping rules
date: 2026-11-01 · by: infra · issue: 0002
`tests/test262.toml` lists `[[group]]`s checked in order, first match wins, unmatched tests
are active. Order: skip, scan-eval, later, so tests that need dynamic code count in
scan-eval even if they are also later. Features implied by includes
(`harness/features.yml`, e.g. isConstructor.js → Reflect.construct) count for matching.
The scan covers the test and its includes, ignores comments and needs a non-identifier
character before the pattern; scan-eval holds direct and indirect eval, the Function /
GeneratorFunction / AsyncFunction / AsyncGeneratorFunction constructors, and the
`language/eval-code` and `built-ins/eval` dirs. Additions to the TESTING.md skip set:
`CanBlockIsTrue/False` flags, `__getter__`/`__setter__`, `import.meta`,
`intl-normative-optional`, `built-ins/AbstractModuleSource`, tests calling
`$262.createRealm(` without the cross-realm tag, and the sloppy-only raw test
`language/comments/hashbang/use-strict.js`. `later` additionally holds `built-ins/Date`
(not in the P0-P2 builtin list) and `__proto__`; `Symbol` and `Symbol.iterator` stay active
because for-of, spread and destructuring (P1) need them.

## D1023: Ratchet file semantics
date: 2026-11-01 · by: infra · issue: 0002
A failures file (`path: reason`, sorted) covers whatever tests ran: `--expected` fails on
failing tests that are not listed, reports listed tests that passed (stale) without
failing, and ignores entries for tests outside the run. `--update` replaces the entries of
the tests that ran and keeps the rest, so filtered runs can update it. Reasons are
informational and are not compared.

## D1040: Number conversion algorithms and the shared power-of-five table
date: 2026-11-01 · by: num · issue: 0004
`djs_core::num` uses Ryu (ported from the `ryu` crate) for shortest decimal output, and
QuickJS' `js_dtoa` algorithms on a small fixed-size bignum for everything that needs exact
digits (toFixed/toExponential/toPrecision, toString(radix)), so corner cases match qjs.
Decimal parsing is Clinger's fast path, then Eisel-Lemire (ported from `core::num::dec2flt`),
then an exact bignum slow path. Ryu and Eisel-Lemire share one table of 128-bit powers of five,
stored as every 27th entry plus 2-bit corrections (800 bytes instead of ~21 KB for the two full
tables); unit tests rebuild every entry exactly. The API takes already-converted integer
arguments, reports RangeErrors as an enum with the qjs message, and writes into stack buffers
(no allocation).

## D1041: Decimal strings are always correctly rounded (qjs truncates after 38 digits)
date: 2026-11-01 · by: num · issue: 0004
qjs keeps only the first 38 significant digits of a decimal string or literal and drops the
rest without rounding, so e.g. `Number("9007199254740993." + "0".repeat(30) + "1")` is
9007199254740992 in qjs but 9007199254740994 in V8. djs rounds correctly for any length, like
V8 and SpiderMonkey; the spec allows either. Results differ from qjs only for inputs with more
than 38 significant digits that are within 10^-38 (relative) of a halfway point. For parseInt
with a radix that is neither 10 nor a power of two (where the spec allows approximation) djs
matches qjs exactly: about 128 bits of leading digits, the rest truncated.

## D1090: Copying collector
date: 2026-11-01 · by: heap · issue: 0009
`Heap::collect` is a Cheney copy into a fresh arena, called only explicitly at a safepoint.
Allocation never collects; it sets `gc_pending` once the arena reaches the threshold
`live + min(max(live, 256 KiB), (cap - live) / 2)`. That is twice the live size, but
collections come sooner near the cap. The threshold is policy, kept outside the arena and
recomputed on restore. The to-space is reserved up front with the full from-space size,
which bounds live data. If the host refuses, `collect` returns `OutOfMemory` with the heap
untouched; after that nothing can fail. The arena cap is not exceeded by the to-space, but
host memory peaks at about twice the arena during a collection. Forwarding sets
`gc_bits::FORWARDED` and writes the new offset into the first payload word; every cell
kind is at least 16 bytes (asserted). References to flattened ropes are short-circuited to
the flat string; unflattened ropes are not flattened. The result is breadth-first from the
roots in slot order, so the layout is a function of the graph: collecting twice gives
identical bytes. Nothing is keyed by arena offsets (atom index: content hash; property
index: atom id), so no table is rehashed. Tracing comes only from the `Value`/`Ref` field
types in `cells.rs`, so new cell kinds are collected automatically.

## D1091: Handle table
date: 2026-11-01 · by: heap · issue: 0009
`Roots::HANDLES` points to a `HandleTable` cell: `LEN`, `FREE` (free list head,
`index + 1`), `LIVE`, and a tail of 16-byte `HandleSlot { VALUE, NEXT }` records
(`NEXT = u32::MAX` marks a live slot). `Handle(u32)` is the slot index, stable across
collections and snapshots. `new_handle` / `get_handle` / `set_handle` / `drop_handle`;
using a dropped handle panics. Handles (and the arena roots) are the only references
that survive `collect`; raw `Obj`/`Str`/`Sym`/`Value`s held in Rust are stale afterwards.

## D1080: Speculative parsing never emits bytecode
date: 2026-11-01 · by: lexer · issue: 0008
Every parse ambiguity is resolved before emitting code: by a token-level scan (arrow
heads with an early bailout, destructuring assignment with a memoized bracket scan,
`for` heads), or by speculation that only moves the lexer (TypeScript type skipping). A
checkpoint is `(Lexer, Token, prev_end)`. The only rewrite of emitted code is removing
the last instruction for lvalue conversion. This replaces ARCH technique 2's
"emit and truncate", because every future piece of compiler state would otherwise have to
be part of the checkpoint. See `docs/compiler.md` §3.

## D1081: Name resolution runs once per compilation unit, children first
date: 2026-11-01 · by: lexer · issue: 0008
A free name in an inner function can only be classified (outer binding or global) once
every enclosing function has seen all of its declarations (hoisting), so a function
cannot be resolved "when it ends" (ARCH technique 3). Resolution runs after the whole
unit is parsed, over the functions in reverse creation order (children before parents,
as QuickJS's `js_create_function`), with no recursion. Resolving a child marks
ancestor bindings captured and adds capture entries to the functions in between.
§5.3.

## D1082: Bytecode encoding and the instruction table
date: 2026-11-01 · by: lexer · issue: 0008
`u8` opcode plus little-endian, unaligned immediates (`u16` slots and argc, `u32`
constants and atoms, `i32` jumps relative to the next instruction). P0/P1 jumps are
fixed-width (no short forms until benchmarks justify a relaxation pass). One `ops!`
macro declares name, immediates, stack effect, fuel cost and flags. Encoders, the
decoder, the disassembler, the stack-depth checker and `OPS_HASH` (snapshot build hash)
are generated from it; the interpreter's agreement with it is checked in debug builds.
Phase-1 code uses pseudo-ops (`name_*` with a side table of name refs, `label`,
`enter_scope`/`leave_scope`) that one linear re-emit removes. §4.

## D1083: Captured bindings live in heap Box cells
date: 2026-11-01 · by: lexer · issue: 0008
The resolver turns every captured binding into a local slot holding a Box cell, created
when its scope is entered. Closures copy Box references. Frames never hold anything a
closure points into, so there is no `close_loc` (unlike QuickJS), and frames can be
suspended and snapshotted freely. Per-iteration bindings come from a new Box per
iteration (`rebox` for C-style `for`). §5.4.

## D1084: Exception handler tables and gosub/ret for finally
date: 2026-11-01 · by: lexer · issue: 0008
Each function template has a table of `(start, end, handler, depth)` try regions,
innermost first. Unwinding searches it per frame (callers with `ret_pc - 1`), so the
value stack holds no catch markers (unlike QuickJS). `finally` blocks are compiled once
and entered with `gosub` at a fixed depth (one value slot plus the return address, an
int32 Value). §7.4.

## D1085: Re-entrant ops finish immediately, slow paths call privileged JS
date: 2026-11-01 · by: lexer · issue: 0008
An op never resumes mid-way. Its fast path is leaf Rust. If user code is needed
(getter, setter, ToPrimitive, iterator protocol), it pushes a call frame whose header
records one of a few fixed continuations (`PushResult`, `DiscardResult`,
`ConstructResult`, `Entry`) and completes when the callee returns. Multi-step slow paths
are privileged JS builtins (`%ToPrimitive`, `%GetIterator`, ...). That needs a minimal
privileged compile mode (`%Name(...)` intrinsic calls) and a hand-written `prelude.js` in
P1, before bgen exists (P2). This replaces ARCH's "small numbered resume state" for
multi-step ops. §7.3.

## D1086: Frame records in the index-addressed value stack
date: 2026-11-01 · by: lexer · issue: 0008
The value stack is a growable arena `ValueArray` addressed by index. A frame is
`callee, this, args (padded to the formal count), a 5-slot header (prev_bp, ret_pc,
argc, flags/continuation, new_target, as int32 Values), locals, operand stack`. Calls do
not copy arguments. The GC traces the stack as plain values, and interpreter registers
are flushed to an arena `Interp` cell whenever `run` returns. §7.1.

## D006: Accept compiler design note 0008 (docs/compiler.md)
date: 2026-11-01 · by: coordinator · issue: 0008
docs/compiler.md is the implementation reference for the compiler, bytecode and
interpreter. Its [ARCH] proposals (D1080, D1081, D1084, D1085, privileged mode in P1) are
applied to ARCH.md. 0006 is split into 0013 to 0016 following its section 10.

## D007: Internal-reference Value tag
date: 2026-11-01 · by: coordinator · issue: 0008
Add a Value tag for references to internal cells (function templates, Boxes, …) so they
can sit in object slots. Such values never reach JS code; the GC and `verify` trace them
like other references. Implemented by the first issue that needs it (0015 or P1-1),
which updates value.rs, the GC and verify together.

## D008: Codemode top level is a function body
date: 2026-11-01 · by: coordinator · issue: 0008
pi wraps codemode scripts as `(async (tools, console) => {<code>\n})`
(pi-private/packages/codemode/src/runtime/vm.ts). So top-level declarations are locals,
`this` is lexical (the global `this`), and `tools`/`console` are parameters. The async-body
mode matches that; dquickts' global-eval async semantics are not what pi uses.

## D009: Keep source text for Function.prototype.toString
date: 2026-11-01 · by: coordinator · issue: 0008
Scripts keep their source (one-byte when possible). Function templates record their span,
and `toString` slices it. A compile flag may strip the source later if memory matters.

## D1100: Difftest compares what pi's codemode shows the model
date: 2026-11-01 · by: infra (decided with the coordinator) · issue: 0010
Every difftest engine first runs one shared JS prelude (`tests/corpus/prelude.js`) that
defines `console.*`, `text()`, `exit()`, `store`/`load` and fake `tools` with the
formatting of pi's codemode prelude (pi-private `packages/codemode/src/runtime/
prelude-source.ts`): console arguments as strings raw, Errors as name/message/stack,
everything else `JSON.stringify` with a `String()` fallback, joined by spaces. Return
values are serialized in the VM as pi does (a string is shown as it is, anything else as
compact JSON, an unserializable value fails the script) and uncaught errors are shown
the way pi's host formats them. qjs's native console.log (JS_PrintValue) is never shown
to the model, so djs does not replicate it, and `djs run` in djs-cli may use a simple
printer of its own. The case body is the body of a strict
`async (tools, console) => {...}` in a global Script, as in pi (D008; ours is strict, as
djs always is): `this` is the global object and there is no `arguments`. Stack frame lines are dropped before
comparing for now. pi's lockdown (freezing the built-ins), its `tools` Proxy and
`image()` are left out: they would need Proxy and regex in djs and add nothing to the
comparison.

## D1101: djs-difftest reuses djs-test262's library
date: 2026-11-01 · by: infra · issue: 0010
`djs-difftest` depends on the `djs-test262` crate for its parallel map, ratchet file
handling, globs and Markdown tables instead of a third shared crate; both are std tool
crates that change together. Anything engine-specific (the qjs driver, the transcript)
stays in each crate.

## D1102: Importing dquickts' tests
date: 2026-11-01 · by: infra · issue: 0010
`scripts/difftest-import-dquickts.py` generates `tests/corpus/dquickts/` (checked in,
with QuickJS's LICENSE): one case per test function, with the helpers it references
(transitively) and its call. Tests that cannot work in a strict codemode script (sloppy
mode, qjs's `std`/`os` modules, module loading, stack positions of eval'd code) are
listed in the script with reasons and not generated. Since a codemode script cannot
call `std.evalScript`, each single-line `assertSyntaxError(src)` in
`test_typescript.ts` becomes its own case whose source is `src` (expected: a
SyntaxError), and other `evalTS` lines are dropped. Cases that use eval or Function get
`// difftest: skip-djs` (a non-goal for now).

## D1120: Native JSON leaf (parse and stringify fast path)
date: 2026-11-01 · by: heap · issue: 0012
`json::parse` (heap string) and `json::parse_utf8` (host bytes) implement JSON.parse without
a reviver. The lexer is monomorphized per encoding (Latin-1, UTF-16, UTF-8). It reads heap
input in place through a borrow that is re-taken after each allocation, so the input is
never copied. Containers use an explicit stack: the pending keys and values of all open
containers live in Rust vectors, and each object or array is created in one allocation
when it closes (`Heap::new_object_from_entries` / `new_array_from_values`). Intermediate
values in Rust memory are safe because allocation never collects. Small integral numbers
use the int32 form. Errors use QuickJS' messages and positions (line at `\n`, column in code
points). Where qjs prints a raw UTF-8 byte (`Unexpected token '…'` after `-`) we print
U+FFFD, which is what qjs's message decodes to. Nesting deeper than 5000 levels is
`SyntaxError: stack overflow` for parse and `InternalError: stack overflow` for stringify;
qjs's limits depend on its C stack. `json::stringify_fast` handles plain data only and
returns `Slow` before any observable effect for:
- enumerable accessors,
- an object-valued `toJSON` anywhere on the chain (functions do not exist yet, so any
  object might be callable; primitive `toJSON` values are ignored, as the spec does),
- any object class other than ordinary or array,
- BigInt and host refs.
Cycle and depth errors found by the fast path are final, because everything before them
was plain data. Its output buffer is bounded by the string limit and the heap's remaining
room.

## D1130: The bytecode table and emitter API
date: 2026-11-01 · by: lexer · issue: 0013
`bytecode/ops.rs` declares every opcode once:
`Name method(imm: Kind, ...) pop N [+ imm K] push M cost Class [flags ...];`. The macro
generates `Op` (`repr(u8)`, table order, which the interpreter matches on), `OPS:
&[OpInfo]` (name, immediate kinds, size, stack effect, cost class, flags THROWS / CALLS /
PHASE1 / JUMP / BRANCH / TERM / GOSUB / ALLOC), and one typed emitter method per opcode
on `CodeBuf<M>`. The label mode is a type: `CodeBuf<Phase1>` keeps labels symbolic (the
`label` pseudo-op, label ids in `Target`s), `CodeBuf<Final>` patches `i32` offsets
relative to the end of the instruction on `finish`. The resolution re-emit uses
`CodeBuf::with_labels(n)` to reuse the phase-1 label ids. Decoding, disassembly and the
stack checker are table-driven with no per-opcode code. `OPS_HASH` (FNV-1a over the
table) changes with any table edit. `build_hash(LAYOUT_HASH, OPS_HASH)` is meant for heap
images that contain bytecode.

## D1131: Code cells
date: 2026-11-01 · by: lexer · issue: 0013
`heap/cells.rs` gains kinds 12 to 16: `FuncTemplate`, `Script`, `BoxCell` (the Box of
compiler.md 5.4; named so it does not shadow `alloc::boxed::Box`), `Bytes` (raw bytes,
untraced) and `RefArray` (traced refs to non-JS cells). Code, the line table and the
handler table (16-byte records) are `Bytes` cells read in place; constants are a
`ValueArray`, children a `RefArray`, capture descriptors and names `U32Array`s.
`bytecode::alloc_template` writes a template from its parts after running the stack
checker, which also fills in `STACK`. None of this needs the internal-ref Value tag
(D007). Only putting templates or Boxes into object slots does, which is 0015's job.

## D1110: Annex B regex grammar in non-unicode mode
date: 2026-11-01 · by: num · issue: 0011
Supersedes the "no Annex B" wording of issue 0011 for regular expressions (agreed with the
coordinator). Without the `u` and `v` flags, patterns use the Annex B.1.2 grammar exactly as
QuickJS implements it: lone `{`, `}` and `]`, identity escapes such as `\_`, legacy octal
escapes and `\8`, `\c` without a letter, class escapes in ranges (`[\w-.]`), `\k` without named
groups, and quantified lookaheads. Engines apply this grammar regardless of strict mode and
real-world patterns rely on it, so it does not conflict with "always strict". `u` and `v` mode
patterns stay strict. The legacy RegExp statics (`RegExp.$1`, test262 feature
`legacy-regexp`) remain out of scope.

## D1111: Regex engine design
date: 2026-11-01 · by: num · issue: 0011
`djs_core::regexp` parses a pattern into a small tree (regexes only; ARCH's "no AST" is
about the JS compiler) and compiles it to a `Vec<u32>` program: a header (flags, capture
count, register count, search and anchored entry points), the group names, then code with
absolute jump targets. The execution model follows QuickJS: one backtrack stack of `u32`
words with frames (pc, position, previous frame | kind, generation) and an undo log of
capture and register writes above each frame; a slot is logged at most once per frame
(per-slot generation marks). All state is integers, so running out of fuel at any
instruction returns an `ExecState` (serializable with `to_words`) that `resume` continues.
Fuel is one unit per instruction, plus the length for backreferences and one per character
skipped by the first-character prefilter (`SCAN`) at the head of the search loop. The stack
limit is a parameter (`ExecLimits`) and overflowing it is a result, not an allocation failure.

## D1112: `\p{...}` is not supported yet
date: 2026-11-01 · by: num · issue: 0011
Unicode property escapes need large tables (General_Category, Script, Script_Extensions,
binary and sequence properties). Until they land behind the `regexp-unicode-props` cargo
feature (follow-up), `\p`/`\P` in `u`/`v` mode are syntax-checked like qjs and then rejected
with the SyntaxError "unicode property escapes are not supported". In non-unicode mode `\p`
is the identity escape `p`, as in qjs.

## D1113: Where the regex parser deliberately differs from qjs
date: 2026-11-01 · by: num · issue: 0011
Duplicate named groups follow the spec: an error unless the two groups are in different
alternatives of some disjunction (MightBothParticipate). qjs only checks a coarse
"alternative scope" counter and accepts nested or sequential duplicates such as
`(?<a>x)(?:y|(?<a>z))`. We also do not copy qjs' implementation limits: up to 65535
capture groups (qjs: 254), no group name length limit (qjs: about 120 bytes), no register
limit; nesting deeper than 255 groups or classes is "stack overflow". Everything else,
including error messages, matches qjs (checked by the corpus).

## D1114: Case mapping tables
date: 2026-11-01 · by: num · issue: 0011
`scripts/gen-unicode.py` also writes `unicode/case_tables.rs` from UnicodeData.txt,
CaseFolding.txt and SpecialCasing.txt (Unicode 17.0.0, D1031): simple upper and lower case
mappings as runs of equal delta with stride 1 or 2 (203 and 185 runs, 8 bytes each), simple
case folding as 28 exception runs over lowercase, unconditional multi-character upper (102)
and lower (1) mappings as `[u16; 4]`, and `Cased` (156 ranges) and `Case_Ignorable` (459)
for Final_Sigma. About 6.6 KB in total; the regex engine uses about 4.1 KB of it. ASCII is
handled in code. Context and language dependent mappings are left to callers (Final_Sigma
is a String.prototype.toLowerCase rule). `unicode::case` exposes the lookups and in-order
iteration over mapped code points.

## D1170: djs-bgen is Rust with its own parser for the subset
date: 2026-11-01 · by: infra · issue: 0017
djs-bgen is a std Rust crate (`crates/djs-bgen`) with no dependencies: a hand-written
recursive-descent parser for the builtin subset (and for the declarations in
`intrinsics.d.ts`), a typed AST, its own type checker, the resume-point analysis and the
emitters. Reasons: the subset is small and explicitly typed, so its type rules (branded
numerics, Value/Obj/Str/Sym/Fn, intrinsic signatures) are ours to define anyway, and the
Rust backend later needs exactly this typed AST. A parser that only accepts the subset is
itself most of the syntax check, with our own positions and messages. Regenerating needs
only cargo, so the gate can always run the freshness check. Rejected: oxc (a large
dependency tree in every workspace build, and full-TS parsing means rejecting most of
what it parses), and the TypeScript compiler API on node (TypeScript 7, which pi-private
uses, ships only the native `tsc` with no JS API; we would have to pin 5.x and need node
and npm to regenerate). `builtins/tsconfig.json` still lets editors and any `tsc`
type-check the sources, but bgen never depends on it.

## D1171: Builtin subset rules
date: 2026-11-01 · by: infra · issue: 0017
The subset djs-bgen accepts (builtins/README.md, crates/djs-bgen/src/check.rs):
top-level functions with explicit types only; locals declared with a type and an
initializer, no shadowing; assignments and `++`/`--` are statements on locals; calls only
by name to intrinsics or builtins (user code through `Call`/`Construct`); no property
access, object/array literals, `new`, closures, `arguments`, `eval`, generators, async,
classes, `switch`, labels, `typeof`/`in`/`instanceof`, `==`/`!=`, `??`, `**`. Conditions
must be booleans. Numerics: `+ - *` on one integer type, `/` and `%` only on f64,
bitwise operators only on i32 (`>>>` gives u32), no mixed numeric types except implicit
i32/u32 → f64 widening on assignment and argument passing, and integer literals adapt
when they fit. Integer overflow is a bug in the builtin: the Rust backend will check it
and raise an internal error, and the JS backend does not check. A builtin that takes
`this` cannot be called directly from another builtin. `intrinsics.d.ts` brands
i32/u32 with an optional property so that tsc catches i32/u32 mixing but accepts
literals; bgen is stricter than tsc on everything numeric.

## D1172: Generated prelude format and installation
date: 2026-11-01 · by: infra · issue: 0017
`builtins/gen/prelude.js` is one strict Script: an IIFE that declares every builtin as a
local function (builtins call each other directly; nothing leaks into the global
object), in file-name order and then declaration order, followed by installation calls:
`%InstallMethod(target, key, fn, length)` (method on the object at the dotted path
`target` from the global object; writable, configurable, not enumerable; `name` = key)
and `%InstallIntrinsic(name, fn)` (makes `fn` callable as `%name(...)` from privileged
code and from interpreter slow paths, for builtins tagged `@intrinsic`). Both are leaf
intrinsics declared in `intrinsics.d.ts`. This is a proposal for P1-2, the first
consumer: getters, symbol keys, constructors and other attributes are added when needed.
`builtins/gen/resume-points.txt` (the `--report`) is checked in next to it, so reviews
see when a builtin becomes reentrant.

## D1173: Reentrant intrinsics are single-step
date: 2026-11-01 · by: infra · issue: 0017
Following D1085, an intrinsic tagged `@reentrant` runs a leaf fast path and calls at
most one user function whose result completes it (`Get`, `GetV`, `Set`, `HasProperty`,
`CreateDataPropertyOrThrow`, `DeletePropertyOrThrow`, `Call`, `Construct`). Operations
with several steps around user code (ToPrimitive, GetMethod, ToNumber of objects,
LengthOfArrayLike, ArraySpeciesCreate) are written in the builtin language. Error texts
follow QuickJS where cheap, and messages that need engine-internal data (such as a
function's internal name) come from leaf intrinsics (`NewNotConstructorError`).

## D1190: Benchmark harness
date: 2026-11-01 · by: infra · issue: 0019
`crates/djs-bench` is a dependency-free runner (no criterion): warmup (two calls or
100 ms), then samples for about a second (5 to 50); the median sample is the recorded
value and the best is printed. Results go to `tests/scores/bench.tsv` (date, commit,
machine, benchmark, value, unit) with `scripts/bench.sh --record`; `--compare [REV]`
flags changes beyond 10% (0.5% for sizes) and exits 1 on regressions. Machines are
identified by hardware model, architecture and CPU count (or `$DJS_BENCH_MACHINE`), and
comparisons prefer rows of the same machine. The benchmark scenarios are the ones the
component issues measured, so numbers stay comparable. Timing is not in the gate; quick
mode (every benchmark once, small inputs) runs as a unit test so the gate keeps the
harness working.

## D1191: Engine benchmarks run in the difftest codemode model
date: 2026-11-01 · by: infra · issue: 0019
`tests/bench/*.js` run through djs-difftest's `Engine` trait with the shared prelude,
so the qjs engine already exists and the djs engine that 0016 adds to difftest is
reused unchanged. A sample is a whole run; for qjs that includes the process start,
measured by `empty.js` and subtracted. Each script declares its expected result
(`// bench: expect ...`, recorded from qjs), and a run with any other result is not
counted.

## D1192: Wasm size probe
date: 2026-11-01 · by: infra · issue: 0019
Until djs-wasm exists, the size track builds `tests/bench/size`, a cdylib that calls
djs-core's main entry points, for wasm32-unknown-unknown with the `wasm` profile. It is
its own Cargo workspace (with a copy of `[profile.wasm]`), so workspace builds and the
gate never build it. Reported: file size, size without custom sections, gzip -9, and
`wasm-opt -Oz` when available.

## D1180: Inbound wire format, version 1
date: 2026-11-01 · by: num · issue: 0018
docs/wire.md is the specification. It is a tagged, little-endian, one-value format with a
4-byte header (`DJW` + version). Numbers are int8, int32 or float64. Strings carry their
UTF-16 length up front so the VM allocates them at their final size. They are UTF-8 (ASCII
when bytes == units, copied as is), or UTF-16 for strings with lone surrogates. Lengths are
LEB128 varints, which may be zero-padded so the encoder can reserve space before
`encodeInto` reports the byte count. The key table is inline: the first use of a key carries
the string and later uses an index, so encoder and decoder are both one pass. Arrays may hold
holes. `Uint8Array` decodes to an ordinary array of numbers until typed arrays exist. Tag
`0x3F` (host ref) is reserved and rejected. The encoder takes plain data only (plain or
null-prototype objects, arrays, `Uint8Array`, primitives). It never calls `toJSON`. Other
objects, functions, symbols, BigInt and cycles are a TypeError that names the path. Shared
references are copied. Benchmarks (in the spec) show it beating `JSON.stringify` +
`encodeInto` + `json::parse_utf8` on every dataset tried, so it replaces JSON as the inbound
path.

## D1181: The gate runs node
date: 2026-11-01 · by: num · issue: 0018
`scripts/gate.sh` runs the `js/` package's tests (`node --test js/test/*.test.ts`) and the
wire cross-language test (node writes a corpus into a temporary directory, then
`cargo test -p djs-core wire_cross -- --ignored` decodes it). It needs node >= 22.6 for
TypeScript type stripping. The `js/` package has no dependencies and no build step. Its
`tsconfig.json` type-checks `src/` (`tsc -p js`); the tests and benchmarks use node's APIs
and are only type-stripped, since there is no `@types/node` without dependencies.

## D1200: Tests that run subprocesses use generous timeouts
date: 2026-11-01 · by: infra · issue: 0020
Tests that shell out to qjs (or any engine) give every case a generous wall-clock limit
(60 s, hang protection only) and bounded parallelism (4 workers). The gate runs while
other agents build and test, and a millisecond run can stall for over a second.
A case that is meant to hang gets its own short limit (a `// difftest: timeout`
directive, or a separate run) and is never the reason another case times out. Timeouts
are reported as `timeout: no result within Ns`, not as an output difference.

## D1150: libm is the math dependency of djs-core
date: 2026-11-01 · by: heap (approved by the coordinator) · issue: 0015
`libm` (pure Rust, no_std, a port of musl's libm) is the only allowed runtime dependency
for math in djs-core: `**` now, `Math.*` later. `core` has no `pow`, `exp`, `sin`, ...
Number::exponentiate handles the spec's special cases before calling `libm::pow`,
because C's pow differs from JS there: any `x ** NaN` is NaN, `x ** ±0` is 1, and
`(±1) ** ±Infinity` is NaN. qjs on macOS uses the system libm, which can differ from
musl in the last bit for some inputs. That is acceptable (the spec says
implementation-approximated), so difftest failures of that kind are expected; recognize
them as such rather than "fixing" them.

## D1151: Interpreter state in the arena
date: 2026-11-01 · by: heap · issue: 0015
The value stack is `Roots::STACK` (a `ValueArray` whose `LEN` is its capacity, addressed
by index, grown by copying). The `Interp` cell (`Roots::INTERP`, kind 17) holds the
registers `BP HP SP PC`, the run state (`Idle Running HostCall Done Threw ThrowPending`),
the parked host call id and continuation, and the completion value. The realm
(`Roots::REALM`) is an internal reference (D007, tag 13) to a `Realm` cell (kind 18) with
the global object and the intrinsics by index. Pending host calls are `HostCall` cells
(kind 19) listed in `Roots::HOST_CALLS`. Frame records follow D1086, but the header has 6
slots, `prev_bp prev_hp ret_pc argc flags new_target`. `prev_hp` is needed because a
frame's header position depends on its padded argument count, which is only recorded in
that header. Closures are objects of class `Function` (slot 0 = template, slots 1.. =
Boxes, all internal refs); host functions are class `HostFunction` (slot 0 = fn id).
Error instances are class `Error`. Since every register is an index, a suspended or
parked VM is fully described by its heap bytes, and `Vm::from_bytes(vm.bytes())`
continues where it stopped.

## D1152: Host call protocol and fuel semantics
date: 2026-11-01 · by: heap · issue: 0015
Calling a `HostFunction` (from bytecode, from a getter or setter, or directly with
`Vm::call`) records a `HostCall` cell `{id, fn_id, this, args}`, pops the callee,
`this` and the arguments, parks with `pc` after the call, and `run` returns
`HostCall { call_id, fn_id }`. While parked, `run` returns the same status again.
`resolve_host_call(id, Ok(v))` applies the call's continuation: push `v`, or drop it for
setters, or `Done(v)` for an entry call. `Err(e)` throws `e` at the call instruction
(handler lookup with `pc - 1`). Fuel: each instruction charges its `OPS` cost class.
`run` stops with `OutOfFuel` at the first instruction boundary where the fuel is used up,
so resuming re-executes nothing. Safepoints are back jumps, frame entry and return, and
the start of `run`. There the stack above `sp` is cleared, the heap is collected if it
asks (or always with `gc_stress`), and the cached code and stack offsets are re-derived.

## D1210: Exported heap layout drives the host-side reader
date: 2026-11-01 · by: num · issue: 0021
`heap::layout::layout_json()` serializes `LAYOUTS` (cells by name: kind, min size, fields,
tail), the header, the roots magic, `LAYOUT_HASH`, the value tags, atom id bits, header flags,
object classes (from `ObjectClass::from_u32`) and the stored property flag bits (including
the accessor bit). The private constants this needs (`atom::INT_BIT`/`SYM_BIT`/`INDEX_MASK`,
`object::STORE_ACCESSOR`) are now `pub(crate)`. `js/src/layout.generated.ts` holds that JSON.
`layout_generated_ts_is_current` keeps it fresh; regenerate it with
`DJS_UPDATE_LAYOUT=1 cargo test -p djs-core layout_generated` after changing cells, tags or
classes. A drift test fails if a valid value tag is missing from the export.
`js/src/reader.ts` takes every offset and constant from that file and checks the magic and
layout hash before reading.

## D1211: Outbound reads are trees; strings via node's Buffer
date: 2026-11-01 · by: num · issue: 0021
`readValue(arena, bits)` decodes like JSON does. Shared objects are read per reference and a
cycle is a TypeError with its path. A value budget (`maxValues`) stops exponential expansion of
sharing. Memoizing objects to keep sharing cost 30%. Holes stay holes. Ropes are walked, so
the VM need not flatten. Only own enumerable string-keyed data properties are read.
Accessors, which cannot be run from the host, are left out. Symbols, functions and other
classes, BigInt and host refs become `Opaque` placeholders. On node, strings are decoded with
`Buffer.latin1Slice` / `ucs2Slice`, which are exact and run at memcpy speed. A portable path
uses only `TextDecoder` and `String.fromCharCode`: windows-1252 only when there are no bytes
in 0x80..0x9F, and a fatal UTF-16 decoder with a fallback for lone surrogates. The gate runs
the reader's cross-language test (extends D1181).

## D1140: The compile() entry point and template conventions
date: 2026-11-01 · by: lexer · issue: 0014
`compiler::compile(heap, src: &[u8], &CompileOptions { goal, filename, params }) ->
Result<Ref, CompileError>` returns a `FuncTemplate` without captures (plus its `Script`
cell, with the source kept per D009). `Goal::Script` (D004) or `Goal::AsyncBody` (D008:
`params` such as `["tools", "console"]` become argument slots). `CompileError { kind:
Syntax | OutOfMemory, message, pos, line, col }` uses QuickJS wording; the embedder makes
the SyntaxError object. `FuncTemplate::FLAGS` is `bytecode::func_flags`: the kind in the
low 4 bits (`KIND_SCRIPT`, `KIND_ASYNC_BODY`, then the P1 function kinds), `ASYNC`,
`GENERATOR`. The embedder calls the top level with `this` = the global object in both
modes. The function reads it with `special_object THIS` (new kind 4) into a local in the
prologue, as for any `this` pseudo-binding (compiler.md 5.7).

## D1141: Script-mode globals in bytecode
date: 2026-11-01 · by: lexer · issue: 0014
A script with top-level declarations starts with `global_decls idx`. `CONSTS[idx..]` holds
int32 Values `n_var, n_func, n_let, n_const` and then the atom ids in that order
(`bytecode::global_decls`). The op performs GlobalDeclarationInstantiation: check
everything, then create the bindings. Top-level `let`/`const` initializers use
`init_global_lex`; every other access to a top-level name is `get_global`/`put_global`,
and these check TDZ at run time (so does `get_global_typeof`). An assignment to a
`const` (local or global) compiles to a TDZ read when one is needed, then
`throw_const_assign`. A read of an undeclared global `undefined` compiles to the
`undefined` op (the global is non-writable and non-configurable).

## D1142: P0 compiler scope and structure
date: 2026-11-01 · by: lexer · issue: 0014
P0 compiles every statement and expression that needs no functions: literals (objects
without methods/accessors, arrays with holes and spread, untagged templates), all
operators including optional chains and logical assignment, calls and `new`, `if`,
`while`, `do`, `for(;;)`, `switch`, labels, `break`/`continue`, `throw`, blocks, and
`var`/`let`/`const` with TDZ checks. Everything else is a SyntaxError
"<construct> not supported yet". Reference-ness is tracked by an `Expr` kind returned
from the expression parser (not by inspecting the bytecode), so `(0, o.m)()` correctly
calls with `this` = undefined. The depth limit is 1000, pinned by a 1 MiB-stack test.

## D1240: Function objects get name, length and prototype from their template
date: 2026-11-01 · by: lexer · issue: 0024
The compiler emits no code for function object properties. `fclosure` (the interpreter)
defines `length` (`FuncTemplate::LENGTH`, the parameters before the first default or
rest) and `name` (`NAME`, "" when null). Both are {writable: false, enumerable: false,
configurable: true}. For `KIND_NORMAL` functions it also defines `prototype`, a new object
with a non-enumerable `constructor`. Arrows (`KIND_ARROW`) have no prototype and are not
constructors. The compiler fills in `NAME` statically, by NamedEvaluation of anonymous
functions and arrows in declarators, simple assignments, object literal properties and
parameter defaults. `PARAMS` excludes a rest parameter.

## D1241: Parameters and per-iteration bindings
date: 2026-11-01 · by: lexer · issue: 0024
Simple parameter lists keep parameters in their argument slots; a captured parameter is
copied into a Box in the prologue. Once a default or a rest parameter appears, all
parameters become locals, initialized in order by the phase-1 code (`get_arg`, the
default under `is_undefined`, the new `rest index` op for `...r`), with TDZ between
them. `for (let ...)` loops emit the phase-1 pseudo-op `iter_scope s` after the init and
before each update. The resolver turns it into `rebox` for each captured binding of the
scope, and into nothing otherwise.

## D1230: The djs-wasm ABI: integer exports, staging/out buffers, numbered handles
date: 2026-11-01 · by: num · issue: 0023
djs-wasm exports only i32 functions and imports nothing; there is no wasm-bindgen. Data
crosses the boundary through two buffers in linear memory:
- host → wasm: the *staging* buffer (`djs_staging(len) -> ptr`) carries sources, names,
  wire data, JSON, images and argument handle lists;
- wasm → host: the *out* buffer (`djs_out_ptr/len`) carries error messages, display text,
  value bits and host call records.

VMs and heap handles are numbered from 1 (handle index + 1). 0 means `undefined` as an
argument and failure as a result, with the reason in the out buffer. Misusing the ABI (a
freed VM or handle, an out-of-range staging length) is a panic, and so a trap. The
version is `djs_abi_version()` = 1, and the host checks it. Instead of a `vm_bytes` copy,
the image is the arena at `vm_arena_ptr/len`; the reader uses the same view.
`vm_from_bytes` takes over the staging Vec as the new arena, without a copy. Panics
record their message through a hook (`djs_panic_ptr/len`), then trap. The host discards
the instance, which is one per sandbox (`new WebAssembly.Instance` costs about 30 µs), so
a trap never takes other sandboxes down. Full table: docs/host.md.

## D1231: djs-wasm uses std and its built-in dlmalloc
date: 2026-11-01 · by: num · issue: 0023
djs-core stays no_std. The export layer uses std on wasm32-unknown-unknown, for
`thread_local!` state, the panic hook and the default allocator (Rust's bundled dlmalloc).
This adds no dependency, dlmalloc is about 6 KB of code, and the engine's own
allocation is one `Vec<u8>` arena per VM, so allocator speed hardly matters. `talc` would
save a few KB but adds a dependency; revisit if size gets tight. On wasm32 the panic
hook does not chain to std's default hook, which keeps the formatting machinery for
stderr out of the module (11 KB).

## D1232: The wasm profile uses opt-level "s", not "z"
date: 2026-11-01 · by: num · issue: 0023
At `z`, LLVM stops inlining the tiny arena accessors (`Arena::write::<u32>`,
`layout::read_array`), and they dominate every profile. djs-wasm in node 25, medians:

| opt-level | stripped | gzip -9 | interpreter loop | wire decode 10k objects | JSON parse 10k objects |
| --- | ---: | ---: | ---: | ---: | ---: |
| z | 304 KB | 116 KB | 135 ms | 15.7 ms | 24.0 ms |
| s | 359 KB | 132 KB | 80 ms | 5.4 ms | 9.0 ms |
| 2 | 393 KB | 143 KB | 80 ms | 4.5 ms | 7.6 ms |
| 3 | 403 KB | 145 KB | 81 ms | 4.4 ms | 7.4 ms |

`s` costs 16 KB of gzip over `z` and makes the interpreter 1.7x faster and decoding
about 2.9x faster. Speed is part of G2. Follow-up: `#[inline(always)]` on the hot
accessors, then re-measure at `z` and `s`; if `z` gets close in speed, `z` may come back.
The 400 KB size budget is meant after `wasm-opt -Oz`, which is not installed here yet.
Until it is, the size track reports the size without custom sections and the gzip size.

## D1250: try/catch/finally compilation
date: 2026-11-01 · by: lexer · issue: 0025
`try` compiles per compiler.md 7.4 (D1084). Before the try block, a token scan decides
whether a `finally` follows (emission-free lookahead, D1080), so `break`/`continue`/
`return` inside know to `gosub` it. Handler entries are appended as each region closes:
try region → catch (or the rethrow stub), catch region → the rethrow stub when there is a
finally. That gives innermost-first order. The finally block runs at depth base + 2
(value slot and return address) on every path. The parser tracks the statement-boundary
operand depth (`stmt_depth`) for handler depths and for unwinding jumps. The catch
parameter shares its scope with the catch block (redeclaring it with `let`/`const`/
function is an error). `var` of the same name is allowed and assigns the parameter, as in
qjs, which implements Annex B.3.4 here.

## D1220: Script-mode globals at run time
date: 2026-11-01 · by: heap · issue: 0022
The global lexical record (D1141) is a hidden ordinary object without a prototype at
`Realm::GLOBAL_LEX`, never visible to JS. Each `let`/`const` is a data property whose
value is `UNINITIALIZED` until `init_global_lex`. Bindings are configurable, so the init
can overwrite them through the normal define path; `writable` is false for `const`.
`get_global`, `get_global_typeof` and `put_global` look there first. While a binding is
uninitialized they throw `ReferenceError: x is not initialized`; writing to a const throws
`TypeError: 'x' is read-only`. Then they fall through to the global object.
`global_decls` follows QuickJS (`JS_CheckDefineGlobalVar`) rather than the spec's
VarNames list:
- a lexical name conflicts with a non-configurable own property of the global object, or
  with an existing lexical binding (`SyntaxError: redeclaration of 'x'`);
- a var name conflicts with a lexical binding, and needs the global object to be
  extensible if it is new (`TypeError: cannot define variable 'x'`).
All checks run before any binding is created. Vars become `{undefined, writable,
enumerable, non-configurable}` properties of the global object. Variables created by
`var` are non-configurable, so "previous script had `var x`" is caught the same way as in
qjs.

## D1221: End-to-end corpus with qjs-generated expectations
date: 2026-11-01 · by: heap · issue: 0022
`tests/data/e2e/*.js` are small programs, Script mode by default; a first line
`// mode: body` makes a codemode body with params `(tools, console)`.
`gen-expected.sh` runs each case in qjs, in strict mode, with a `console.log` shim that
formats values like `vm::display` (strings raw, Errors `Name: message`, `-0` / `NaN` /
`Infinity` via `String`, everything else `JSON.stringify`). Uncaught exceptions become a
last line `Uncaught ...`. `crates/djs-core/tests/e2e.rs` runs every case three ways and
each must match the `.expected` file exactly:
- plain;
- suspension: fuel 1 for the first 5000 stops, then 997-fuel slices, with a
  `Vm::from_bytes(vm.bytes())` round trip at every stop and host call;
- GC stress.
`DJS_E2E_MODE=plain|suspend|gc` runs one mode. Uncaught non-Error values should be thrown
in body cases: qjs's stderr printer formats objects differently from the shim.

## D1270: Intrinsic ids
date: 2026-11-01 · by: lexer · issue: 0027
`call_intrinsic id, argc` takes `id` as the index into `bytecode::intrinsics::INTRINSICS`.
The table lists the `declare function`s of `builtins/intrinsics.d.ts` in declaration
order (kind Leaf/Reentrant by tag), then the `@intrinsic` builtins of `builtins/*.ts` in
prelude order (kind Builtin: JS functions registered by `%InstallIntrinsic`). The .d.ts
stays the single source of truth. A unit test checks the hand-maintained Rust table
against it and prints the replacement on drift, so no generator step is needed.
`INTRINSICS_HASH` belongs in the build hash: ids are part of the bytecode format.
Privileged code checks intrinsic names and arities at compile time.

## D1271: Object literal compilation
date: 2026-11-01 · by: lexer · issue: 0027
Methods, getters and setters are separate function kinds (`KIND_METHOD`/`KIND_GETTER`/
`KIND_SETTER`: no `prototype`, not constructors) defined with `define_method`/
`define_getter`/`define_setter`. Static keys name the function at compile time (getters
"get x", setters "set x"). Computed keys emit `set_function_name prefix` before the
`_elem` define, as does a computed data property whose value is an anonymous function or
arrow. A literal `__proto__: v` (identifier or string key, not computed, not shorthand)
compiles to `set_proto`. Duplicates are an early error, except in destructuring
patterns. `super` inside methods stays "not supported yet" until classes add home
objects (P1-7). `delete a?.b` deletes the chain's last reference and yields `true` when
the chain short-circuits. Spread arguments build an array (`array_from k`, then
`array_spread`/`array_push`) for `call_spread`/`construct_spread`.

## D1160: djs in difftest provides the codemode globals natively for now
date: 2026-11-01 · by: infra · issue: 0016
djs cannot run `tests/corpus/prelude.js` yet (it needs JSON, Promise, Map,
Object.freeze, classes, ...). Splitting the prelude would be churn that disappears once the
builtins exist, so djs-difftest's djs engine defines `console.*`, `text`, `exit`,
`store`, `load`, `tools` (`echo`, `fail`) and `ALL_TOOLS` as host functions that follow
the prelude's formatting rules exactly (pi's `format`, `outputText`, the return value
and the uncaught error; not `Vm::display`, which prints `-0`, `NaN` and `function`
differently). A test runs cases djs can already run on both engines and requires
identical transcripts. Known differences until djs has promises: `tools.*` return their
result directly instead of a promise, and `tools.fail` throws at the call. Values that
need JSON.stringify's slow path (toJSON, getters) are shown as visible
`<djs: ...>` markers. Once djs runs the prelude, the engine switches to it and this
glue goes away.

## D1161: Stress variants in difftest and the gate
date: 2026-11-01 · by: infra · issue: 0016
`djs-difftest --engine djs --suspend-every N` (continue every N fuel in a VM restored
from the heap image) and `--gc-stress` (collect at every safepoint) run each case again
next to the plain djs run. A transcript that differs from the plain run is a hard
failure, independent of `.expected` and the ratchet. The gate runs `--engine djs
--ratchet --suspend-every 97 --gc-stress` in a debug build: 0.27 s today. The cost grows
with fuel/N × heap size and with live objects × safepoints, so N (and possibly a subset for
GC stress) may need to change as more cases pass. Out-of-process engines get no stress
variants. Ratchet reasons name the uncaught error (`throws ReferenceError: ...`) when
the engine threw one the expectation does not contain; `--clusters` groups failures by
the same key.

## D1280: Slow paths replace an operand and re-execute the instruction
date: 2026-11-01 · by: heap · issue: 0028
A new continuation `Replace(slot)` (in frame flags: kind in bits 0-3, slot in bits 8-30;
parked host calls use the same encoding with the entry marker in bit 31) stores the
callee's result into an absolute stack slot of the caller and resumes at `ret_pc`.
Before dispatch, `primitive_operands` checks every operand that an op converts with
ToPrimitive (arithmetic, bitwise and relational: `number`; `+` and loose equality with a
non-nullish primitive: `default`; `to_string`, `concat` and property keys: `string`).
It goes left to right, which is the spec's evaluation order. For the first operand that
is an object, the interpreter calls the installed `%ToPrimitive(operand, hint)` with
`Replace(operand slot)` and `ret_pc` = the same instruction. When it returns, the
instruction runs again with a primitive in place. The op itself never sees objects there,
so no per-op resume state exists (D1085 holds), and only `%ToPrimitive` is needed from
the prelude. `get_method` / `get_elem_method` through a getter use the same continuation
to fill a placeholder below the receiver, resuming after the instruction. Without a
prelude (`VmOptions::prelude = false`), an object operand is a TypeError.

## D1281: Prelude installation
date: 2026-11-01 · by: heap · issue: 0028
`Vm::new` compiles `builtins/gen/prelude.js` (included at build time) with
`CompileOptions::privileged_script("<prelude>")` and runs it to completion before
returning; `VmOptions::prelude` (default true) turns that off. Dispatch:
- `call_intrinsic id, argc` dispatches on the D1270 table, using ids computed at compile
  time from names.
- Leaf and reentrant intrinsics are Rust in `interp/run/intrinsics.rs`. `Call` and
  `Construct` reuse the stack layout of `call`/`construct`; `Get`/`Set` call an accessor
  with `PushResult` (Set pushes its `undefined` result below the setter frame).
- Builtin-kind ids call the function registered by `%InstallIntrinsic`, from a
  `Realm::INSTALLED` table indexed by id. Calling one that isn't installed is an
  InternalError.

`%InstallMethod` resolves its dotted target from the global object, falling back to the
realm's prototypes for `Object/Function/Array/Error.prototype` while those constructors
don't exist. It defines the method (writable, configurable, not enumerable), sets
`name`/`length`, and flags the function `OBJ_BUILTIN`. Cost, `Vm::new` with vs without
the prelude: native release 92 vs 7 µs; djs-wasm in node 25 146 vs 11 µs; heap
36 KB vs 17 KB. That is at ARCH's 100 µs budget before the prelude has grown, so the
base snapshot (copy an image instead of compiling) is the next step for startup. The e2e
test already starts every case from one shared base image.

## D1282: Error stack traces
date: 2026-11-01 · by: heap · issue: 0028
Errors the VM creates (runtime errors, `%NewTypeError` / `%NewRangeError` /
`%NewNotConstructorError`) get an own, non-enumerable data property `stack` at creation,
in QuickJS's format. It has one line per frame, `    at name (file:line:col)\n`, innermost
first, at most 10 (qjs's default `Error.stackTraceLimit`). The current frame is
positioned at the faulting pc, callers at `ret_pc - 1`, through the template's line
table. Names: the template `NAME`, `<anonymous>` when null, `<eval>` for the top level.
Prelude frames are hidden, except functions installed by `%InstallMethod`, which show as
`    at map (native)` like qjs's C builtins. qjs exposes `stack` through an accessor; an
own data property is simpler and prints the same. `djs run` prints the stack after
`Uncaught Name: message`.

## D1070: The djs engine in djs-test262
date: 2026-11-01 · by: infra · issue: 0007
djs-test262's `djs` engine runs in-process with a fresh `Vm` per test. A setup Script
defines `$262.evalScript`, then the harness files and the test run as Scripts in the
same realm (`this` = the global object), each compiled separately so that a parse error
names its file. `print`, `$262.global`, `$262.gc` (a no-op for now) and
`$262.detachArrayBuffer` (throws a TypeError until ArrayBuffer exists) are host
functions or values. `$262.evalScript(src)` is
`function (src) { return $262.__compileScript(src)(); }`: the host function compiles
`src` as a Script in the same VM while the call is parked (compiling only allocates)
and returns the function, which the wrapper calls. So declarations land in the global
object and the global lexical scope, the completion value is whatever the compiled
Script returns, and a syntax error is a catchable SyntaxError. One difference: the
Script's top-level `this` is undefined instead of the global object, because the
wrapper calls it plainly. The thrown value's constructor name is read from data
properties only (`constructor.name` through the prototype chain, else `name`); the host
never runs JS. Limits: 5e9 fuel per test (the slice loop also checks the runner's
wall-clock limit), and the runner's memory limit (512 MiB by default) as the heap cap.
`InternalError: out of memory` is reported as `oom`, and the reason of a timeout says
which limit hit.

## D1071: Unimplemented syntax is not a SyntaxError for test262
date: 2026-11-01 · by: infra · issue: 0007
The compiler rejects constructs it does not implement yet with a SyntaxError whose
message ends in "not supported yet". The djs engine reports those parse errors as
`Unsupported`, not `SyntaxError`, so negative parse tests that expect a SyntaxError do
not pass for the wrong reason. Measured: active passes went from 3,945 to 2,687 and
later from 1,455 to 83. When a feature lands without its early errors, its negative
tests show up as new failures in the ratchet, as they should.

## D1072: test262 in the gate
date: 2026-11-01 · by: infra · issue: 0007
The gate runs the whole active group on djs (20,124 tests) in the debug build that
`cargo test` already produced, against `tests/test262/expected-failures.txt`: 4.2 s
today (0.5 s for active + later in release). The slowest test takes 0.84 s in debug,
against a 20 s wall-clock limit. Most tests stop early today, so the time will grow as
djs passes more. Above about 30 s, switch the gate step to a release build. A
deterministic subset is the fallback. The `later` group is not in the gate.

## D1260: Inlined fixed-size arena accessors; the wasm profile stays at opt-level "s"
date: 2026-11-01 · by: num · issue: 0026
The field accessors (`layout::read_array`/`write_array`, `FieldType::read/write`,
`Arena::read/write/get/set/len`) are `#[inline(always)]` and use `first_chunk` for
fixed-size loads and stores; the out-of-bounds panic is a cold function. At opt-level z,
`copy_from_slice` used to stay a call (sometimes a `memcpy` call for 4 bytes). That
alone brought z's interpreter loop from 132 to 83 ms (s: 79 ms).

Re-measured at z and s after all of 0026 (djs-wasm in node 25, medians of 3, with the
0022 builtins):

| | z | s |
| --- | ---: | ---: |
| stripped / gzip | 404 / 152 KB | 461 / 168 KB |
| interp loop | 118 ms | 83 ms |
| wire decode 10k objects | 7.2 ms | 4.3 ms |
| parse_json 10k objects | 13.7 ms | 8.0 ms |
| parse_json 1 MiB text | 6.0 ms | 2.1 ms |
| json_stringify 10k objects | 9.8 ms | 5.8 ms |

z is still 1.4x to 2.8x slower, far from the ~10% bar: the JSON lexer, the decoder and
`resize` zero fills are loops that z leaves unoptimized. As D1232, the wasm profile stays
at "s" (+16 KB gzip). The 400 KB budget is meant after `wasm-opt -Oz` (not installed
here).

## D1261: The arena keeps a zeroed frontier; data is written once
date: 2026-11-01 · by: num · issue: 0026
`Arena` tracks its own `len`. `bytes.len()` (the Vec's length) is a *zeroed frontier* up
to 16 KiB ahead of it, and everything in `bytes[len..]` is zero:
- `alloc` below the frontier is a bounds check and a bump;
- moving the frontier is a cold path (`resize`, a memset at opt-level s and up);
- `alloc_init(size, at, data)` (strings, GC copies) and `alloc_from_iter` (UTF-16 units
  converted on the way in) append their data at the frontier instead of zeroing and then
  copying, so those bytes are written once. The GC's to-space never zero-fills.

`bytes()`, `slice`, `slice_mut`, `copy_within` and `split_mut` stay strict against
`len`. The hot field accessors check only against the Vec (one compare less), which is
still memory safe (an access between `len` and the frontier reads zeros). Debug builds
also assert `len`, so tests catch stray accesses. `shrink_capacity` truncates the
frontier, so images and restored arenas behave as before. Natively, GC is 11-19% faster
and object decode 7% faster; in wasm, decode is 15% and JSON parse 10% faster.
Measured: a static zero block or doubling `extend_from_within` instead of `resize`
avoids z's byte loop but costs 15% natively, and only `slice::fill` becomes
`memory.fill` at z, so `resize` stays.

## D1262: Short decimals format without Ryu
date: 2026-11-01 · by: num · issue: 0026
`num::format::shortest` first tries `short_decimal`: for k = 1..8, `m = round(a * 10^k)`
is accepted when `m as f64 / 10^k == a`. The search stops once `ulp(a) * 10^k >= 1`, and
`None` falls back to Ryu. It gives exactly Ryu's digits:
- the division is correctly rounded with exact operands, so the test is exactly "the
  decimal round-trips";
- below that ulp bound, at most one multiple of 10^-k lies in the round-trip interval
  (also for the asymmetric intervals of powers of two), so the first k is the unique
  shortest form;
- a candidate near a half-way point cannot round-trip, so rounding in `a * 10^k` never
  hides the right candidate.

Checked against Ryu on 2M random, short-decimal, power-of-two and near-2^53 values, and on
a 525k-case qjs corpus. Effects:
- `num/to_string(short decimal)`: 32 to 11 ns;
- stringify of 500k numbers: 12.3 to 8.4 ms natively, 24.9 to 13.3 ms in wasm;
- random doubles pay about 1 ns for the failed tries.
## D1310: Builtin installation forms
date: 2026-11-01 · by: infra · issue: 0031
Extends D1172. JSDoc tags on builtins (crates/djs-bgen/src/check.rs, emitted by
emit_js.rs as leaf intrinsic calls declared in intrinsics.d.ts):
- `@builtin A.b.c` (repeatable, so `Number.parseInt` can be the global `parseInt`):
  `%InstallMethod`. A bare name or `globalThis.x` targets the global object.
- `@constructor Name` (+ `@extends Parent`): `%InstallConstructor(target, key, f, length,
  parent)` defines the global, reuses the realm's prototype for that name if it has one
  (Object, Function, Array, the error prototypes) or creates one inheriting from the
  parent's prototype (or Object.prototype), sets `f.prototype` (all attributes false),
  `prototype.constructor`, `name`, `length`, and with a parent f's [[Prototype]].
  `new.target` is an expression of type Value, allowed only in `@constructor` builtins.
- `@getter A.b.k` / `@setter A.b.k`: `%InstallAccessor(target, key, f, isSetter)`, one
  half of a configurable, non-enumerable accessor, named `get k` / `set k`.
- `@value A.b.k` (+ `@attributes writable enumerable configurable`, default none): the
  zero-parameter function is called once at install time, `%InstallValue(target, key,
  f(), attrs)`. Namespace objects (`globalThis.Math`) are values too.
- Builtins may take a rest parameter `...args: Value[]` (a local of type Obj: an array).
Installs onto the global object come first (in source order), then the rest, so dotted
paths resolve. Symbol keys and `%Name%` targets come with 0029/0030.

## D1311: Builtins are not constructors; bound functions
date: 2026-11-01 · by: infra · issue: 0031
Prelude functions are ordinary functions, so `%InstallMethod`/`%InstallAccessor` strip
their `prototype` (a heap-internal removal; it is non-configurable) and flag them
`OBJ_BUILTIN`. `IsConstructor` of an `OBJ_BUILTIN` function is true only with
`OBJ_BUILTIN_CTOR` (set by `%InstallConstructor`). `Function.prototype.bind` creates an
`ObjectClass::BoundFunction` (7; slots: target, bound this, an Array of bound arguments).
`call` and `construct` unwrap it on the stack (bound arguments inserted before the
caller's); constructing one constructs the target with new.target = the target, which is
what the spec does when new.target is the bound function itself (no Reflect.construct
yet). `instanceof` uses the target. Name and length are set by the builtin, since reading
the target's may run getters. `Function` exists as a constructor that throws a TypeError
("not supported yet"), so that `Function.prototype` and `constructor` links work.

## D1312: Error constructors and stacks
date: 2026-11-01 · by: infra · issue: 0031
The Error constructors are builtins (builtins/error.ts): GetPrototypeFromConstructor
(`%RealmIntrinsic("TypeErrorPrototype")` as the fallback), `%ErrorCreate(proto)` (an
`ObjectClass::Error` object), `message` (ToString), InstallErrorCause, then
`%CaptureStack(o)`. The stack follows D1282 but leaves out the prelude frames at the top
of the stack (the constructor itself), as qjs does, and is defined last, so own keys come
in qjs's order apart from `stack` itself (qjs has an accessor on Error.prototype). The
VM's own errors already use the same realm prototypes, so `e.constructor === TypeError`.
EvalError, URIError and AggregateError prototypes are new realm intrinsics.
AggregateError itself waits for iteration (IterableToList). djs-test262's
`$262.evalScript` now calls the compiled Script with `this` = the global object (D1070's
one difference is gone).

## D1320: VM images carry a build hash (Roots::BUILD)
date: 2026-11-01 · by: num · issue: 0032
`vm::BUILD_HASH = build_hash(build_hash(LAYOUT_HASH, OPS_HASH), INTRINSICS_HASH)`.
`Vm::new` writes it into a new `Roots::BUILD` field (offset 48; bare heaps keep 0).
`Vm::from_bytes` and `Vm::from_base` check it first and fail with
`LoadError::BuildMismatch { found, expected }` ("the image is from another djs build"):
images hold bytecode and intrinsic ids, which mean nothing to another build.

`Roots::VERSION` stays the layout hash, which the JS reader checks. If VERSION carried the
build hash, every new op or intrinsic would change `layout.generated.ts`. `LoadError`
gained a `Display` impl.

## D1321: The base snapshot (djs-prelude)
date: 2026-11-01 · by: num · issue: 0032
`crates/djs-prelude` has a `build.rs` that uses a host build of djs-core: `Vm::new` (realm
plus prelude), `collect`, a full `Vm::from_bytes` verification, then it writes
`$OUT_DIR/base.img`. The library embeds it as `djs_prelude::BASE`, and
`djs_prelude::new_vm(opts)` returns `Vm::from_base(BASE, opts)`.

Cargo reruns the build script whenever djs-core is rebuilt, and that includes
`builtins/gen/prelude.js` (via `include_str!`).

`Vm::from_base` copies the image into a fresh arena with `Heap::from_bytes_trusted`:
header and build-hash checks, but no walk over every cell. It also requires an idle VM
with no handles. Skipping the walk is safe enough: the base was verified at build time,
and a bad cell would still be a panic, not UB.

The image is deterministic. Tests check that building it twice gives the same bytes, and
that `BASE` equals a fresh build. A node test checks that the host-built base is
byte-identical to the realm djs-wasm compiles inside wasm32 (so nothing depends on pointer
width). That export, `vm_new_compiled`, sits behind the `compiled-prelude` feature, which
only the gate's debug build turns on.

The embedders use the base: djs-wasm `vm_new`, djs-cli, difftest, test262 and the e2e tests.
`Vm::new` (compile at startup) stays for unit tests and for making the image.

What lives in the base:
- the atoms;
- the realm: the global object, intrinsics and builtins;
- the compiled prelude;
- an idle interpreter state.

Nothing per VM. Host functions, globals and handles are added after the copy. `after_copy`
is the hook for per-VM state such as a future `Math.random` seed.

Startup, medians:

| | before | after |
| --- | ---: | ---: |
| `vm_new`, native | 74 µs | 0.5 µs |
| `vm_new`, wasm (node 25) | 144 µs | 0.4 µs |
| base size | | 36 KB |

The release djs-wasm shrank from 578 to 564 KB even with the base embedded, because the
code that builds the realm is no longer linked. After 0031 part 1 the base is 72 KB:
compiling takes 216 µs natively and the copy 0.8 µs.

Instantiating the wasm module (about 35 µs) is now most of a sandbox's startup.

## D1313: Primitive wrappers and property lookup on primitives
date: 2026-11-01 · by: infra · issue: 0031
Boolean, Number, String and Symbol objects are `ObjectClass` 8-11 with the primitive in
slot 0 (`realm::wrap_primitive`; `%ToObject` and `%WrapPrimitive` create them). A String
object's index properties (read-only, enumerable) and `length` are ordinary own
properties defined at creation, which costs O(length) per ToObject of a string; the
interpreter's own string fast paths never create wrappers. Boolean.prototype,
Number.prototype and String.prototype are wrappers of false, +0 and "" (realm
intrinsics, like Symbol.prototype). Property reads on primitives (`lookup`) go through
the realm prototype for the type, and getters get the primitive as `this`.
`%PrimitiveValue` unwraps for thisBooleanValue and friends. The `Symbol.for` registry is
a prototype-less realm object from keys to symbols; `keyFor` checks the entry under the
symbol's description.

## D1314: Math.random
date: 2026-11-01 · by: infra · issue: 0031
xorshift64* (as QuickJS), 52 bits into [0, 1). The state is a realm intrinsic (an
internal ValueArray of two int32 halves), so it is heap data: a restored snapshot
continues its sequence. `VmOptions::random_seed` (0 = a fixed default) is applied by
`Vm::new` and by `Vm::after_copy` (D1321), so every VM from the base starts from the
host's seed; `Vm::set_random_seed` reseeds a running VM.

## D1315: Symbol keys and attributes in installation tags
date: 2026-11-01 · by: infra · issue: 0031
Extends D1310. A path may end in a well-known symbol: `@builtin
Symbol.prototype[Symbol.toPrimitive]`, `@value Math[Symbol.toStringTag]`; bgen emits the
key as `%WellKnownSymbol("toPrimitive")` and the install intrinsics accept string or
symbol keys (a symbol method is named `[description]`). `@attributes` also applies to
`@builtin` (emitted as `%InstallMethod` followed by `%InstallValue` with the attributes;
`none` means all false). One function may be several accessor halves
(`%ThrowTypeError%` is Function.prototype's `caller` and `arguments` getter and setter);
the first install names it.

## D010: djs-core at opt-level 1 in the dev profile
date: 2026-11-01 · by: coordinator · issue: 0034
Debug test runs exercise the engine heavily: the e2e suspension mode reloads and verifies
the heap image at every stop, and gc-stress copies all live cells at every safepoint. At
opt-level 0 the e2e test took ~48 s; at opt-level 1 it takes ~3.4 s (measured by infra).
Debug assertions and overflow checks stay on. The whole gate now runs in ~46 s.

## D1340: The prelude keeps no source text
date: 2026-11-01 · by: infra · issue: 0034
A privileged compile (`CompileOptions::privileged`, only the prelude) stores no source
text in its Script (D009 keeps it for everything else). Installed builtins print as
`[native code]` and the other prelude functions are unreachable, so nothing reads it.
The base image went from 174 to 127 KB and `djs_prelude::new_vm` from 2.3 to 1.4 µs.
It is the existing flag rather than a new field, so the compiler change is two lines
(the lexer agent works there). The e2e suite barely changed (49.7 to 48.1 s in debug):
its cost was the verified image load per suspension stop, which D010 (djs-core at
opt-level 1 in dev) addressed.

## D1341: Array builtins follow the spec's Get, with leaf fast paths
date: 2026-11-01 · by: infra · issue: 0034
builtins/array.ts implements Array.prototype step by step. Where qjs reads elements with
a HasProperty probe first (`at`, `with`, `toReversed`, `toSorted`, `toSpliced`) or writes
back only the moved elements (`sort`), djs follows the spec; that is visible only through
proxies, and behavior.js compares results there instead of trap logs. Fast paths that
cannot be observed:
- `%Get` and `%CreateDataPropertyOrThrow` try the dense element store first (an own dense
  element is a plain data property; for an Array, defining one is the same as [[Set]]).
- `%ArraySpliceDense` does splice's element moves (shift, unshift, splice) in one step on
  a plain dense Array: no holes, every element dense, extensible, writable length, no
  index properties on the prototype chain. Otherwise the generic moves run.
- `sort`: the present elements are collected, undefined set aside, and a stable
  bottom-up merge sort runs in the builtin language with the comparator; without one,
  `%SortPrimitives` sorts lists of non-symbol primitives in Rust (by ToString, code
  units, stable). Objects without a comparator take the generic path (their ToString
  may run user code).
Builtin-language loops run at interpreter speed: 200k elements sorted with a comparator
take 1.8 s (qjs 0.04 s); a user loop of 3.4M calls is 0.6 s vs 0.06 s, so the call path
is the bottleneck, not the sort.

## D1342: String builtins and their leaves
date: 2026-11-01 · by: infra · issue: 0034
builtins/string.ts implements String.prototype (except match, matchAll, replace,
replaceAll, search, split: num's, 0033) plus substr/trimLeft/trimRight (Annex B; the
trim aliases are the same functions), String.fromCharCode, fromCodePoint and raw. The
work is in leaves in crates/djs-core/src/builtins/string.rs that read the flat view of a
string (one rope flatten, then one-byte or two-byte units in place): substring (a
one-byte source copies bytes; results narrow to one byte when they can), indexOf /
lastIndexOf (a plain scan), case mapping (an ASCII fast path that returns the same
string when nothing changes; else the full SpecialCasing mappings of 0011 with
Final_Sigma, no language rules, so toLocale*Case are the same), trim, well-formedness,
repeat (doubling with rope concatenation), fromCharCode/fromCodePoint.
`localeCompare` is QuickJS's without Intl: the difference of the first differing code
points (qjs normalizes to NFC first). `normalize` has no Unicode normalization tables
yet: it returns strings that are already normalized without them (all code units below
U+0300 for NFC, below U+00A0 for the other forms) and throws a TypeError ("not supported
yet") for anything else; the tables are a follow-up. IsRegExp looks only at
Symbol.match until RegExp objects exist (RegExp.prototype has @@match, so regexps
answer true). `String.prototype[Symbol.iterator]` waits for the iterator intrinsics.

## D1343: instanceof through InstanceofOperator when it matters
date: 2026-11-01 · by: infra · issue: 0034
The `instanceof` op keeps its Rust OrdinaryHasInstance for the common case: the target
inherits Function.prototype's own Symbol.hasInstance (or there is none, without a
prelude) and its `prototype` is a data property; a bound function is checked through its
target chain. Otherwise (a custom or non-callable @@hasInstance, a getter, a `prototype`
accessor) it calls the prelude's `%InstanceofOperator(v, target)` (builtins/function.ts,
`@intrinsic`) with a PushResult continuation, so user code runs as an ordinary call.
%ThrowTypeError% (Function.prototype's `caller`/`arguments` accessors) is set up by
`%InstallAccessor` as the spec has it: name "", length 0, both non-configurable, and not
extensible.

## D1344: djs-bgen's behavior test has a timeout
date: 2026-11-01 · by: infra · issue: 0034
Following D1200, `crates/djs-bgen/tests/behavior.rs` kills qjs after 60 s (hang
protection only; a run takes well under a second) and fails with `timeout: no result
within 60s`. Its output pipes are drained on threads, so a large output cannot block qjs.
A builtin that loops forever in the qjs stand-ins had hung the test for almost two hours.

## D1290: The iteration protocol lives in the builtin language
date: 2026-11-01 · by: lexer · issue: 0029
GetIterator, IteratorClose and the throw-completion variant are `@intrinsic` builtins in
`builtins/iter.ts` (`%GetIterator(obj)`, `%IteratorClose(it)`, `%IteratorCloseThrow(it)`),
not ops: the compiler emits `call_intrinsic` for them, and the step (`next` / `done` /
`value`, with `iter_check_object`) is plain bytecode (D1085: multi-step semantics in
privileged JS). `%GetIterator` returns only the iterator; callers read `next` once,
right after. `%IteratorCloseThrow` calls `return` but swallows its errors and ignores
its result, so the body's exception wins. Array Iterators are an `ObjectClass` with
leaf intrinsics for their state (CreateArrayIterator, ArrayIteratorTarget / Index /
SetIndex / Finish / Kind) plus CreateIterResultObject and CreateArrayFromPair; next()
itself is builtin-language code and reads `length` through LengthOfArrayLike (the spec;
qjs uses ToUint32). `ArrayIteratorTarget` is the brand check ("Array Iterator object
expected") and returns undefined once the iterator is exhausted.

## D1291: `%Name%` install targets in bgen
date: 2026-11-01 · by: lexer · issue: 0029
A `@builtin` path may start with a `%Name%` segment (`%ArrayIteratorPrototype%.next`,
`%IteratorPrototype%[Symbol.iterator]`): the checker accepts it and the emitter passes it
through as the `%InstallMethod` target string, which the runtime resolves to the realm
intrinsic (0030). Only the first segment may be `%Name%`.


## D1345: Builtins over iterables
date: 2026-11-01 · by: infra · issue: 0034
builtins/iterable.ts builds on 0029's `%GetIterator`: IterableToList, Array.from's
iterator path (GetIteratorFromMethod with the method read once; the iterator is closed
with `%IteratorCloseThrow` when mapping or storing throws), Object.fromEntries,
Object.groupBy (the groups live in a prototype-less object from keys to arrays, so the
lookups run no user code; keys keep first-seen order) and AggregateError's `errors`.
A non-object from `next()` throws without closing the iterator, as the spec says (qjs
closes it). RequireObjectCoercible is GetIterator's own TypeError, which gives qjs's
message. Until the Array and String iterators exist (0030 part 4), Array.from reads
arrays and strings directly, which is what their default iterators do; it switches to
the iterator path when they land.
## D1351: array spread: original iteration in Rust, everything else via %ArraySpread
date: 2026-11-01 · by: heap · issue: 0030
`array_spread` (used for `[...x]`, `f(...x)` and `new F(...x)`) handles two cases in Rust:
- a string, by code points;
- an array without holes, but only while its @@iterator (found as a data property
  along the chain) is the installed `%ArrayPrototypeValues` and
  `%ArrayIteratorPrototype%.next` is the installed `%ArrayIteratorPrototypeNext`.

In those cases the iterator protocol is not observable, which is the shortcut qjs takes.
In every other case the instruction becomes a call of the prelude's `%ArraySpread(arr,
src)` (builtins/spread.ts), whose result is pushed. Both checks happen before anything is
mutated. `ArrayPrototypeValues` and `ArrayIteratorPrototypeNext` are tagged `@intrinsic`
as well as `@builtin` only so that their identity can be checked. Once a String iterator
exists, the string case needs the same identity check. Array Iterator objects use slots
target / index / kind. The leaves accept any kind, so the String iterator can reuse the
class with kind 3 (infra).

Error messages show a symbol key by its bare description, as qjs does (`cannot read
property 'Symbol.iterator' of null`, `'q' is read-only`). This is `conv::describe_key`;
function names keep `[desc]`.

## D1346: The String iterator reuses Array Iterator objects
date: 2026-11-01 · by: infra · issue: 0034
A String Iterator is an `ObjectClass::ArrayIterator` object of kind 3 whose target slot
is the string primitive, with the realm's %StringIteratorPrototype% ([[Prototype]]
%IteratorPrototype%, @@toStringTag "String Iterator"). `%CreateStringIterator(s)` makes
one; `%StringIteratorTarget(it)` accepts only kind 3 (TypeError "String Iterator object
expected"), and heap's `ArrayIteratorTarget` rejects kind 3 ("Array Iterator object
expected"), so neither `next` accepts the other's iterators. The index, finish and
iteration logic (by code points) are the shared leaves plus `%StrCodePointString`.
Fast paths that skip the protocol check identity first, as D1351 does for arrays:
`Exec::iteration_unmodified(v)` is true for an Array or a string whose @@iterator and
`next` are the installed originals (data properties only). Spread's string path now
checks it too, and `Array.from` reads arrays (without a mapping function, which could
grow the array the iterator re-reads) and strings directly only then
(`%HasOriginalIteration`).
## D1350: an interpreter fast path for the hottest simple ops
date: 2026-11-01 · by: heap · issue: 0035
`Exec::run` tries `fast_step` before `step`. It handles get/put/set_loc, get_arg,
push_i8/i32, undefined, drop, dup, inc/dec, to_numeric on a Number, add/sub and the four
relational ops on two int32s, and jump/jump_if_false/jump_if_true on a Bool, then
`continue`s. Anything else returns false with nothing changed, and `step` runs it with
the full semantics. That includes an int overflow, a non-int or object operand (so D1280's
`primitive_operands` is never bypassed), or a non-Bool condition. The reason: every
`exec` arm merges its result through a `Step` (`Result<Option<Status>, Fault>`) that
LLVM keeps in a stack slot, because non-inlined helpers return it through an sret
pointer. The store/reload chain cost more than the ops themselves (loop.js: 555 to
385 ms). Fuel, `pc` and safepoints are charged exactly as `step` charges them, so fuel
counts, suspension points and GC stress are unchanged. A new op belongs in `fast_step`
only if it is simple and stack-only. Keep its semantics identical to its `exec` arm.


## D1292: for-in/of code layout and iterator close
date: 2026-11-01 · by: lexer · issue: 0029
for-in/of loops compile in source order with QuickJS's layout (docs/compiler.md 3.8):
a declaration head stores next to the body (one jump per iteration), an expression head
is emitted at `L_assign` before the right side and jumped around. for-of keeps `it next`
on the operand stack, for-in the key iterator of `for_in_start`. A handler around the
target store and the body closes the iterator with `%IteratorCloseThrow` and rethrows;
the iterator's own steps are outside it. A `break` of the loop closes inline after the
loop; jumps that leave it (labeled break/continue, `return`) `gosub` a close subroutine
after the loop, outside the handler, so that a throwing `return()` propagates instead
of closing twice; `BlockKind::Iter` puts it in the same innermost-first unwind as the
finally blocks (docs/compiler.md 7.4). The subroutine and the inline close are emitted
only when a jump uses them (`BlockEnv::used`). No array fast path yet: `%GetIterator`
and `next()` calls are observable (a patched `Array.prototype[Symbol.iterator]` or
`%ArrayIteratorPrototype%.next`), so a fast path needs a guard on both.

## D1352: more fast paths: leaf intrinsics, floats, dense elements, own-property writes
date: 2026-11-01 · by: heap · issue: 0035
D1350's `fast_step` also handles:
- `call_intrinsic` of IsNumber, IsObject, AsNumber and AsObj (when they hold), Get and
  CreateDataPropertyOrThrow on an existing dense Array element, and `%Call` of a JS
  closure (`fast_intrinsic`);
- add, sub, mul, div and the relational ops on two Numbers (`fast_float`);
- `get_elem`/`put_elem` with an int key on an existing dense element (`fast_elem`).

These rarer cases live out of line. Inlining them made the int32 loop slower, which
shows how sensitive the dispatch loop's register allocation is: measure any addition
with interleaved A/B runs (`work/scratch/0035/ab.sh`). The heap gets two equivalent
shortcuts:
- `Heap::set_own_data_value`, for [[Set]] of an own writable data property (only the
  value changes);
- a new-key branch in `create_data_property` for non-Array objects (a table insert
  without building and validating a descriptor).

## D1347: A stressed difftest run that only times out is reported, not failed
date: 2026-11-01 · by: infra · issue: 0034
Amends D1161. Stress variants (`--suspend-every`, `--gc-stress`) can be orders of
magnitude slower than the plain run: once 0034 made `dquickts/builtin/test_rope.js`
pass (200k rope operations), its stressed runs took 15 minutes in the debug gate, because
the wall-clock limit was only checked between 10M-fuel slices. Now GC stress runs in
20k-fuel slices so the limit (the case's timeout) is honored, and a stressed run that
times out while the plain run finished is listed in the `stress:` line ("timed out, not
compared") instead of failing. Any other difference from the plain run stays a hard
failure.

## D1380: Reflect and construct with an explicit new.target
date: 2026-11-01 · by: infra · issue: 0038
builtins/reflect.ts implements the 13 Reflect functions on the ordinary (and exotic)
internal methods; there is no Proxy yet. New intrinsics: `%ConstructWithArray(f, args,
newTarget)`, `%GetWithReceiver` and `%SetWithReceiver` (OrdinarySet with a receiver;
getters and setters run with the receiver as `this`), and `%DeleteProperty`.
`Exec::construct_with(argc, next, newTarget)` generalizes `construct`: the new object
gets its prototype from new.target, a bound new.target equal to the bound callee becomes
its target, and the explicit new.target is held in a heap handle across the call's
safepoint (D1281's GC hazard) before it is written to the frame. Builtin constructors
already read new.target; `Array` now gives a non-Array new.target's prototype to its
result. Error messages follow qjs: a primitive target is "not a function", a non-callable
object "not a constructor", a callable one "<name> is not a constructor".

## D1381: Map and Set are ordered hash tables rehashed by the collector
date: 2026-11-01 · by: infra · issue: 0038
`heap/collection.rs` implements Map, Set, WeakMap and WeakSet as objects
(`ObjectClass` 12-15) with three slots: the entries, a linear-probing index (a U32Array
of entry number + 1) and the live count. Keys are SameValueZero: numbers are stored
normalized (-0 → +0, one NaN, the int form where it fits), strings are flattened on
insertion and hash by content, objects and symbols hash by offset. Because the copying
collector moves objects, `collect` rehashes every collection it copied, in place (same
index size, no allocation), as a post-pass; so does the Map/Set code itself when it
grows or compacts. (A GC epoch in Roots with lazy rebuilds was considered and dropped:
the counter would make collecting twice change the image, breaking D1090.) Known cost:
every collection rehashes every live table, O(total entries); under `--gc-stress` that
is per safepoint. Map/Set entries are a ValueArray `[forward, removed, k0, v0, ...]`;
deletion leaves a HOLE key. Growth and compaction (more than half deleted, or a clear)
allocate a new array and leave `forward` and the sorted `removed` entry numbers in the
old one, so a Map/Set Iterator (`ObjectClass::CollectionIterator`, 16) that still holds
the old array moves over and shifts its position: entries added during iteration are
visited and deleted ones skipped, as the spec's index-based iteration does. The builtins
(builtins/map.ts, set.ts, weak.ts) call leaf intrinsics (`%CollectionGet`, ...); forEach
and the Set methods walk with `%CollectionIteratorStep`. The Set methods follow the
spec's size-based branches and call the other set's `has`/`keys` as GetSetRecord reads
them.

## D1382: WeakMap and WeakSet are ephemerons in the Cheney collector
date: 2026-11-01 · by: infra · issue: 0038
A WeakMap/WeakSet keeps its pairs in an `EphemeronArray` (cell kind 20, laid out like a
ValueArray). The scan does not trace it; it records the table instead. When the scan
catches up, every untouched pair whose key has been copied gets its key and value
forwarded, and the scan resumes. This repeats until a round changes nothing, so chains
(k1 → v1 holding k2 → ...) and keys reachable only through another table's value stay,
while a key that is reachable only through its own value dies. Pairs with dead keys are
then cleared (HOLE key, undefined value) and the post-pass of D1381 recounts the table.
Tables are processed in copy order and pairs in index order, so the layout is still a
function of the object graph: collecting twice gives identical images (tests in
gc_tests.rs, with image round trips). `verify` checks the cell (pairs, keys objects,
symbols or HOLE). Keys must satisfy CanBeHeldWeakly: objects and symbols not registered
with Symbol.for.
## D1370: The job queue, and when `run` reports Done
date: 2026-11-01 · by: heap · issue: 0037
The microtask queue is `Roots::JOBS`: a `ValueArray` of 5-value entries `[f, a0, a1, a2,
a3]`, read from `Interp::JOB_HEAD`. A job is a call `f(a0..a3)` (`this` undefined) of an
installed builtin, in a bottom frame with the new continuation `Job` (like `Entry`: no
caller, `prev_bp` 0). `run` drains the queue whenever no frame is active: after the entry
call returns, and at the start of a `run` in state `Idle`/`Done`/`Threw`. The entry
call's completion is kept in `Interp::OUTCOME`/`RESULT` while jobs run on top
(`STATE` is `Running` then). `run` returns:
- `Done(v)` / `Threw(e)` once the entry call completed, the queue is empty and no async
  host call is pending (so `Done` means quiescent, as qjs's job loop ends);
- `Idle` when the queue is empty but async host calls are pending (or no call was made);
- `Threw(e)` immediately for an uncaught exception of the entry call or a job, without
  draining the queue first (qjs exits on an uncaught exception without running jobs).
  Only internal errors escape a job: the promise jobs catch everything.
`ThrowPending`'s exception moved to its own field (`Interp::THROW`), so a failing sync
host call inside a job does not overwrite the entry call's result. `Vm::abort` also
drops the queue and the tracked rejections.

## D1371: Promises: Rust slots and reactions, the algorithms in builtins/promise.ts
date: 2026-11-01 · by: heap · issue: 0037
A Promise is `ObjectClass::Promise` (12) with slots state, result, reactions (an internal
`ValueArray` of `[capability, onFulfilled, onRejected]` triples) and flags (handled,
resolved). Leaf intrinsics settle promises (reactions become `%PromiseReactionJob` jobs),
perform `then` and queue thenable jobs; everything that can run user code is in
`builtins/promise.ts`. A promise capability is `undefined` (no result), a Promise, or a
Record `[promise, resolve, reject]`. The Promise form is an internal capability of
`%Promise%`: `NewPromiseCapability(%Promise%)` is unobservable, so it skips the executor
and the two resolving functions, and resolves the promise directly with the promise's
own `[[AlreadyResolved]]` flag. `then`, `finally` and the statics on plain promises thus
allocate one object, not three. Resolving functions for user code (the constructor's
pair) use the same flag; the thenable job makes a fresh pair over a Record, as the spec
does. Messages follow qjs; `Promise.try` follows qjs and the current draft (the callback
runs first, and a promise of the same constructor is returned as is).

## D1372: Builtin closures and internal records
date: 2026-11-01 · by: heap · issue: 0037
The builtin language has no closures, but the spec's promise machinery is full of them
(resolving functions, element functions, finally's thunks). `ObjectClass::BuiltinClosure`
(13) holds an installed intrinsic id and a data value: calling it calls that builtin
with `this` = the data (the interpreter rewrites the frame, like a bound function).
`%NewBuiltinClosure(kind, data, length)` makes one (name "", the given `length`, not a
constructor); `kind` indexes a fixed table in `interp/run/intrinsics.rs`. Shared mutable
state lives in `ObjectClass::Record` (14) objects: prototype-less, internal slots only,
read and written with `%RecordGet`/`%RecordSet`, never handed to user code.

## D1373: Unhandled rejections are tracked in the heap and taken by the embedder
date: 2026-11-01 · by: heap · issue: 0037
HostPromiseRejectionTracker: rejecting a promise that has no handler adds it to
`Interp::REJECTIONS`; a later `then` on it removes it. `Vm::take_unhandled_rejections`
returns (and forgets) the reasons. Embedders check it when `run` returns
`Done`/`Threw`/`Idle` (the queue is empty then), which is when qjs prints `Possibly
unhandled promise rejection: <reason>` and exits with 1; `djs run` does the same, and the
e2e harness reports the first one like gen-expected.sh (qjs's first stderr line).

## D1383: Date reads the host's clock and a fixed local offset
date: 2026-11-01 · by: infra · issue: 0038
The VM never reads a system clock (ARCH "Nondeterminism"). `Vm::set_now(ms)` stores the
current time in the realm (heap data: a restored or replayed VM sees what the host
set); `Date.now()` and `new Date()` read it, and it is 0 until the host sets it. Hosts
set it when they create a VM (djs-cli, difftest and test262 use the wall clock); a
host that wants time to advance during a run sets it again between run slices. Local
time is a fixed offset, `Vm::set_time_zone_offset(minutes east of UTC)`, default 0
(UTC), without daylight saving rules: real time zones need tz data, which djs does not
carry; a host that cares updates the offset. The algorithms are QuickJS's
(`get_date_fields`, `set_date_fields`, `get_date_string`, `js_Date_parse`), ported to
builtins/date.rs, so formatting (toString's `GMT+hhmm` without a zone name, the
toLocale* formats without Intl) and the accepted parse formats match qjs. The difftest
and e2e oracles run qjs with `TZ=UTC` to match the default offset.
## D1374: Async host functions: a promise plus an entry in the pending host call table
date: 2026-11-01 · by: heap · issue: 0037
A HostFunction with the header flag `OBJ_HOST_ASYNC` (`Vm::new_async_host_function`,
`vm_new_async_host_function`) does not park: the call allocates a pending Promise, adds
`{id, fn, args, this, promise}` to the pending host call table (`HostCall::PROMISE`) and
pushes the promise as the call's result. The embedder lists new calls after every `run`
(`Vm::new_host_calls`, a reported bit in `HostCall::FLAGS`; `pending_host_calls` lists
all, for re-issuing after a restore) and completes them with `resolve_host_call` in any
VM state: that removes the entry and queues a `%HostCallSettle` job, which resolves the
promise like a resolve function (thenables adopted) or rejects it. Settling through a
job, not directly, keeps the resolution spec-exact (a `then` getter is user code) and the
order of completions FIFO. While calls are pending and no job is queued, `run` returns
`Idle` (D1370). Everything is heap state, so a VM with pending calls and queued jobs
snapshots and restores like any other. Sync host functions are unchanged.
## D1360: destructuring patterns are parsed after their value (deferred patterns)
date: 2026-11-02 · by: lexer · issue: 0036
A pattern that precedes its value in the source (`let [a] = v`, `[a] = v`, a parameter
or nested pattern with a default, a for-of declaration head) is skipped with a
balanced-bracket scan, the value is parsed and emitted, then the lexer is rewound to the
pattern, which is emitted, and the parser jumps back after the value
(`Parser::deferred_pattern`). Only lexer state moves (D1080), the code needs no jumps
around the pattern, and resolution after parsing makes the bindings visible (in TDZ) to
the initializer. Cost: one extra scan of the pattern's tokens. Errors in the value are
reported before errors in the pattern, which differs from qjs's source order only in
which of two SyntaxErrors a file reports.

## D1361: array destructuring keeps `next it` and reaches it with `pick`/`poke`
date: 2026-11-02 · by: lexer · issue: 0036
Assignment targets are evaluated before the iterator step (spec order), so their
references (0 to 2 values) sit above the iterator. Two new stack ops, `pick n` and
`poke n`, reach under them; their stack effects include the immediate (a new `push M
+ imm K` form in the ops table, also in `OPS_HASH`), so the stack checker verifies
them. "Done" is `undefined` in the iterator slot. The close handler covers the
pattern's own code and not the steps; its depth is relative to a label
(`HandlerLabels::depth_from`) and resolution computes it from the final code with
`bytecode::stack_depths`, since the parser does not track expression stack depth.
Rest elements use the builtin `%IteratorRest`, object rest `%CopyDataPropertiesExcluding`,
and computed keys `%ToPropertyKey` (now `@intrinsic`). No array fast path yet (as D1292).

## D1362: template objects are created on first use and cached in the constant pool
date: 2026-11-02 · by: lexer · issue: 0036
`get_template_object site` reads `CONSTS[site]`: before the first run it holds the
index of a `n, cooked x n, raw x n` block (cooked `undefined` for an invalid escape), and
the interpreter replaces it with the frozen template object (frozen `raw`,
non-enumerable). The constant pool belongs to the function template, so every closure
and every evaluation of the site gets the same object, and a recompiled source gets new
ones (the spec's per-Parse-Node [[TemplateMap]]). Building the object in the compiler
was the alternative; it needs the realm's Array.prototype at compile time.

## D1363: object spread with getters goes through %CopyDataProperties
date: 2026-11-02 · by: lexer · issue: 0036
`copy_data_props 0` still copies plain data properties itself. If the source has an
enumerable accessor, or is a non-empty string, it copies nothing and calls the builtin
`%CopyDataProperties(target, src)` (builtins/spread.ts) with a continuation, as
`array_spread` calls `%ArraySpread`. `{..."hi"}` gives `{0: "h", 1: "i"}` as the spec
and V8 do; qjs (dquickts) gives `{}`.

## D1375: Suspendable frames: frame slices in Generator objects, tail calls for await
date: 2026-11-01 · by: heap · issue: 0037
Generators and async functions share `ObjectClass::Generator` (21): state, the saved
frame (an internal `ValueArray` copy of `stack[bp..sp)`), resume pc, `hp - bp`, the
result promise (async) and the kind. A frame has no absolute stack positions except its
header's caller links, so resuming copies the slice to the top of the stack and rewrites
`PREV_BP`/`PREV_HP`/`RET_PC`/`FLAGS` for the resumer (`PushResult`). Ops (docs/compiler.md
7.7): `generator_start`, `yield`, `async_start`, `await`, `async_return`,
`async_reject`, each naming the hidden local holding the object. `yield` returns the
value to the resumer (`%GeneratorPrototype%.next` etc. in builtins/generator.ts, through
the reentrant `%GeneratorResume`); a resumed `yield` pushes `value isReturn`, so the
compiler runs `return(x)` through its ordinary return sequence (finally blocks). `await`
saves the frame and then replaces it with a call of `%AsyncAwait(state, v)` that keeps
the frame's continuation (a tail call): `%AsyncAwait` does PromiseResolve (user code) and
PerformPromiseThen with the async state itself as the reaction handler, and returns the
result promise. So the first await hands the promise to the caller, and later awaits
hand it to the job that resumed the frame, with nothing special in either.
`%PromiseReactionJob` resumes an async state handler instead of calling it.
`async_return`/`async_reject` tail-call `%AsyncFunctionReturn`/`%AsyncFunctionThrow`,
which settle and return the promise. A throw resumed into a frame with no handler at the
suspension point is thrown by `%GeneratorResume` itself, without restoring the frame.
Fuel: the copies charge one unit per 8 slots.
## D1390: ArrayBuffers and typed arrays in the heap
date: 2026-11-01 · by: infra · issue: 0039
An ArrayBuffer (`ObjectClass::ArrayBuffer`) keeps its bytes in a `Bytes` cell (untraced
data), slot 0 an internal reference (`undefined` once detached), slot 1 the maxByteLength
of a resizable buffer. A resizable buffer's cell is allocated at its maximum, so `resize`
only changes `LEN` and never moves data under live views. A typed array
(`ObjectClass::TypedArray`, one class for all twelve kinds) has the buffer, the kind, the
byte offset and the length (-1: length-tracking); a DataView is its own class. Copying
GC: buffers are copied like every other cell, so a 1 MiB buffer costs a 1 MiB copy per
collection; no large-object space yet (a follow-up if images dominate heaps).
The integer-indexed exotic behavior (10.4.5) lives in the heap's property functions
(`heap/typed.rs`, hooked into `get_own_property`, `find_property`, `has_property`,
`define_own_property`, `delete_own_property`, `own_keys`, `set_own_data_value` and the
element fast paths): a canonical numeric key never reaches a property table or the
prototype chain, element writes convert primitives with ToNumber (the heap has no user
code), and the interpreter's PutElem/PutField first run `%ToNumber` on an object or symbol
value written to a typed array element (the D1280 slow path, isolated in
`primitive_operands`). BigInt64Array/BigUint64Array exist but their constructors throw
"not supported yet" until BigInt.

## D1391: %TypedArray% and the realm's constructors table
date: 2026-11-01 · by: infra · issue: 0039
`@constructor %TypedArray%` installs a constructor that is a realm intrinsic, not a global
(bgen accepts a bare `%Name%`; `%InstallConstructor` stores it in the realm and names it
`TypedArray`), and the concrete constructors `@extends %TypedArray%`. Every installed
constructor is also recorded in a realm table (`Intrinsic::Constructors`), read with
`%RealmConstructor(name)`: the spec's %Name% defaults (SpeciesConstructor, ArrayBuffer for
transfer and slice, the kind constructor of TypedArraySpeciesCreate) must not follow a
replaced global.

## D1392: Bytes on the boundary are Uint8Arrays, read back as copies
date: 2026-11-01 · by: infra · issue: 0039
Wire bytes (`0x30`) decode into a real Uint8Array over a fresh ArrayBuffer (one copy into
a byte cell). The JS reader returns a typed array as a copy of its elements in the host's
typed array of the same kind, and an ArrayBuffer as a copy, never a view of the arena:
collections move cells and wasm memory growth detaches the host's buffer. 1 MiB both ways
costs 0.017 ms to decode and 0.022 ms to read (docs/wire.md); a base64 string of the same
image decodes in 0.1 ms. Uint8Array.fromBase64/toBase64/fromHex/toHex/setFrom* are leaf
code (builtins/typed.rs) and match qjs on every edge case of the corpus case
builtins/base64.js.

## D1420: Function kinds: [[Prototype]] and `prototype` from the ASYNC/GENERATOR bits
date: 2026-11-01 · by: heap · issue: 0042
`fclosure` picks the closure's [[Prototype]] from the template's `func_flags::ASYNC` and
`GENERATOR` bits, for any kind (normal, arrow, method; agreed with the lexer for 0041):
%AsyncFunction.prototype%, %GeneratorFunction.prototype% or
%AsyncGeneratorFunction.prototype%. Every GENERATOR closure gets an own `prototype`
inheriting from %GeneratorPrototype% or %AsyncGeneratorPrototype%; async functions get
none. The three prototypes are realm intrinsics, linked to %GeneratorPrototype% /
%AsyncGeneratorPrototype% (`prototype`/`constructor`, configurable, not writable) at realm
creation. Their constructors are not globals: they are `@intrinsic` builtins in
builtins/generator.ts that throw like `Function` (no source compilation), and Rust wires
them (name, length, `prototype`, [[Prototype]] %Function%, constructor flag) when the
prelude installs them with `%InstallIntrinsic`, because the builtin language cannot pass
functions around as values.

## D1421: Async generators and async iteration on the generator ops
date: 2026-11-01 · by: heap · issue: 0042
No new ops: an async generator is `generator_start` on an ASYNC|GENERATOR function, and its
body uses `yield` and `await` on the same local (docs/compiler.md 7.7 has the sequences:
the yield operand and the return value are awaited by the compiler, as is a `return(x)`
resumption). `await` now suspends with its own state (4, AWAITING) and `yield` with 1, so
`%AsyncGeneratorStep` (builtins/asyncgen.ts), which resumes the frame for a request or for
an awaited promise's job, knows from the state why the frame handed back control: an
await (wait for the job), a yield (settle the oldest request, then go on with the next
one), or a return (complete, drain the queue). Requests are Records `[type, value,
promise]` in an internal queue (slot 6 of the Generator object), handled per spec
(AsyncGeneratorEnqueue/CompleteStep/DrainQueue/AwaitReturn). `for await` uses
`%GetAsyncIterator` (wrapping sync iterators in %AsyncFromSyncIteratorPrototype% objects:
Records with that prototype) and `%AsyncIteratorReturnMethod` / `%AsyncIteratorCloseThrow`
for closing. Job order checked against node; qjs never settles a `next()` queued behind
`return()` on an unstarted async generator (a qjs bug we do not copy).

## D1422: `yield_star` passes the inner result object through
date: 2026-11-01 · by: heap · issue: 0042
For sync `yield*` (requested by 0041): `yield_star loc` is `yield` with the state 6
(suspended raw), and `%GeneratorPrototype%` methods return the yielded value itself
instead of wrapping it in a new iterator result (GeneratorYield ( innerResult )).

## D1330: The build hash includes a hash of djs-core's sources
date: 2026-11-01 · by: num · issue: 0033
Extends D1320. `crates/djs-core/build.rs` hashes (FNV-1a, `\r` dropped) the non-test Rust
sources under `src/` (every `*.rs` except `*tests.rs`) and `builtins/gen/prelude.js` into
`SOURCE_HASH`, and `vm::BUILD_HASH` now combines it with the layout, op and intrinsic
hashes. Before this, the hash covered the cell layouts, the well-known atom and symbol
names, the op table and the intrinsic table, but not the other ids an image depends on: the
realm's slot order (`Intrinsic`), object class numbers, cell flag bits, the meaning of
slots, or the prelude's code. So an old image could load into a build that reads it
differently. Now an image loads only into the build that made it. That was already the
contract (images are not a storage format across djs versions).

Editing a test file does not rebuild the library or the base snapshot. Cargo reruns
djs-prelude's build script whenever djs-core is rebuilt, which was checked by hand (D1321),
so the base cannot go stale. The "Function" for `typeof` that agents saw in djs-cli output
was not an engine bug. The agent tool display capitalizes the first character of a
command's output (`echo function` shows `Function`); `od -c` shows the real bytes.

## D1331: JSON builtins: leaf parse and fast stringify, the rest in the builtin language
date: 2026-11-01 · by: num · issue: 0033
builtins/json.ts implements JSON.parse and JSON.stringify over four leaves
(builtins/json.rs, dispatched from interp/run/json_regexp.rs):
- `%JsonParse(text)`: `json::parse` with the realm's prototypes.
- `%JsonStringifyFast(v, gap)`: `json::stringify_fast`. It returns `null` for `Slow`, a
  value that can never be a stringify result.
- `%JsonQuote(s)`: QuoteJSONString via the fast path on a string.
- `%CreateDataProperty(o, k, v)`: the non-throwing form, for the reviver.

JSON.parse without a callable reviver is one leaf call. The reviver
(InternalizeJSONProperty) runs in the builtin language.

JSON.stringify computes the gap, then tries the fast path when there is no replacer.
Only when that says Slow does it run the spec algorithm (SerializeJSONProperty, Object,
Array, with an internal stack array for cycles) from the start, which is safe because
the fast path has had no observable effect. Wrapper objects are unwrapped with
`%PrimitiveValue`.

Errors keep qjs's messages: the parser's SyntaxErrors, `TypeError: circular reference`,
and `InternalError: stack overflow` from the fast path's depth limit or the VM stack. The
leaves pay fuel afterwards, one unit per 16 code units of input or output, because they
run to completion.

Not done:
- The reviver's third argument `{ source }` and `JSON.rawJSON`/`isRawJSON`
  (json-parse-with-source, test262 "later" group).
- BigInt.
## D1423: Unhandled rejections reach the host; reading a result promise handles it
date: 2026-11-01 · by: heap · issue: 0042
`vm_take_unhandled_rejections` exports D1373's tracker. The Sandbox checks it whenever the
job queue is empty (DONE, and IDLE before waiting on async host calls) and, per
`unhandledRejections`, warns through its console (default), fails the run (`throw`), or
collects the reasons (`collect`). Settled `vm_promise_state` (and
`Vm::mark_promise_handled`) marks the promise handled, as a `then` would: an embedder that
consumes an async body's result promise must not see its rejection reported twice.
## D1393: Builtin constructors do not read newTarget.prototype in the interpreter
date: 2026-11-01 · by: infra · issue: 0039
`construct` used to look up `newTarget.prototype` for the `this` object before calling
any function, including a builtin constructor, which ignores that `this` and calls
GetPrototypeFromConstructor itself at the step the spec gives. With a `prototype` getter
on newTarget that ran user code too early (or threw "not supported yet: getter for
'prototype'"), so argument validation that precedes the prototype lookup (the typed array
`ToIndex`, DataView's checks) came in the wrong order. For an `OBJ_BUILTIN` callee the
interpreter now skips the lookup and passes a plain object; bytecode constructors are
unchanged.

## D1410: The codemode body is an async function
date: 2026-11-01 · by: lexer · issue: 0041
`Goal::AsyncBody` compiles like an async arrow (D008): `async_start`, `async_return` for
every `return`, and a catch-all region that rejects the promise, whether or not the body
uses `await`. So the result of running a body is always a promise: a thrown error
rejects it instead of ending the run with `Threw`. Embedders read the outcome with
`promise_state` (the wasm sandbox already did; the test harnesses in djs-core, djs-wasm
and tests/e2e.rs now do, and e2e prints a rejected body promise as `Uncaught <reason>`,
which is what gen-expected.sh prints for qjs). `djs run --body` reports a rejected body
as an unhandled rejection.

## D1411: A return from a resumed `yield` goes through a throw region
date: 2026-11-01 · by: lexer · issue: 0041
`return(x)` can resume a `yield` deep inside an expression, with operands the parser does
not count on the stack, while finally blocks (`gosub`) and the for-of close subroutines
must run at statement depth. Instead of tracking expression depth, the return path is
`throw` inside a one-instruction handler region whose handler depth is the statement
depth; the handler runs the ordinary return sequence. Iterators opened inside the
statement (array patterns, for-of heads) are closed on the way by a chain of such
regions, one per iterator, with the pattern's relative depth (0036's `depth_from`) or the
for-of's absolute depth. The same throw region carries `yield*`'s returns. The cost is
an exception dispatch per generator `return()` that hits a yield, which is rare. This
also fixes the 0036 follow-up (a pattern's iterator closes when a yield in a default
returns).

## D1412: Generator/async state as an internal local, placeholders in phase 1
date: 2026-11-01 · by: lexer · issue: 0041
The state object of `generator_start`/`async_start` lives in a `VarKind::Internal` local
(`<state>`) of the function, never captured (the ops need a plain `Loc`). Phase-1 code
emits the state ops with `Loc` 0 and resolution rewrites them to the local's slot, like
the name ops. Async arrows have their own state; `await` in a nested arrow body is an
identifier (qjs, ECMA-262 ExpressionBody[~Await]) and in nested arrow parameters a
SyntaxError.

## D011: Promote generators and async iteration to active
date: 2026-11-01 · by: coordinator · issue: 0041
With 0037/0041/0042 landed, `built-ins/{GeneratorFunction, GeneratorPrototype,
AsyncGeneratorFunction, AsyncGeneratorPrototype, AsyncIteratorPrototype,
AsyncFromSyncIteratorPrototype}` and the features `generators`, `async-iteration`,
`Symbol.asyncIterator` move from later to active. Active went from 18,833 / 24,120 to
22,605 / 32,333 (+8,213 tests, +3,772 passing).
## D1430: Privileged code keeps no line tables or local names
date: 2026-11-01 · by: heap · issue: 0043
The prelude's templates have no line table and no local-name table (`NAMES`): prelude
frames are hidden from stack traces or shown as `at name (native)`, and builtins hit no
TDZ errors, so neither is ever read. This saves 55 KB of the 458 KB base image (line
tables 38 KB, names 17 KB). A TDZ error inside privileged code would say `variable is
not initialized`.

## D1431: The base image is compacted before it is collected
date: 2026-11-01 · by: heap · issue: 0043
`Heap::compact_for_image` (heap/compact.rs), called by djs-prelude's build script before
the final collection, reallocates every property table at its live entry count (tables
double as they grow: 46% of the slots were empty), the atom table's entries at their
length, and the value stack at 256 slots without contents. The stack mattered most: it
is traced to its full capacity, so values left above `sp` by the prelude's install run
kept 74 dead templates and their code alive in the image. Base image 403 KB -> 296 KB
(property tables and atoms -52 KB, the stack and what it held -55 KB). Growth after a
restore works as usual (a full table grows on the next insert).

## D1432: No BTree collections and no core slice sorts in djs-core
date: 2026-11-01 · by: heap · issue: 0043
In wasm, `alloc::collections::btree` cost 17 KB of code and `core`'s slice sorts 20 KB
(small-sort networks per element type). djs-core uses neither now: the compiler's
constant dedup and JSON.stringify's deep cycle set are sorted Vecs with binary search;
for-in shadowing checks the objects already visited instead of keeping a set; and the
three sorts go through `crate::sort::sort_by`, one stable merge sort of indexes (a few
hundred bytes per use). Code 612,581 -> 581,874 bytes. Prefer these over new uses of
BTreeMap/BTreeSet and `sort*` in djs-core.

## D1433: Prelude closures carry no `length`, `name` or `prototype`
date: 2026-11-01 · by: heap · issue: 0043
Templates compiled in privileged mode get `func_flags::PRIVILEGED` (1 << 8), and `fclosure`
skips `length`/`name`/`prototype` for them: about 200 prelude helpers are only ever
called, and the builtins user code can see get both from `%InstallMethod`,
`%InstallConstructor` and `%InstallAccessor`, which now define `length` before `name`
(and before `prototype` for constructors), the order qjs and the spec use. Base image
296 KB -> 268 KB.

## D1434: String constants are shared across a compilation unit
date: 2026-11-01 · by: heap · issue: 0043
The compiler deduplicated string constants per function; it now keeps one pool per
compilation unit (strings are immutable, so sharing a cell is invisible). The prelude
repeats messages ("not a function" 26 times) and keys across functions: base -8 KB.
## D1450: Proxies: an empty heap cell, traps in builtins/proxy.ts, same-signature redirects
date: 2026-11-01 · by: infra · issue: 0045
A proxy is `ObjectClass::Proxy` (27; heap/proxy.rs): slots target, handler (both null
once revoked) and flags (callable 1, constructor 2, fixed at creation). The heap cell has
no prototype, no own properties, is not extensible and has `OBJ_INDEXED_PROPS`, so heap
code that does not know proxies sees an empty frozen object: it can never store into
one, and the element fast paths never look past one on a prototype chain.
The interpreter finds proxies with class tests on its slow paths only:
`Heap::lookup_property` / `has_property_or_proxy` report a proxy on the chain (the old
`find_property` / `has_property` treat it as the end of the chain), and `lookup()`
returns `Got::Proxy`. Get, Set, `in`, `delete`, calls, `new`, for-in, object spread and
`instanceof` then call the `@intrinsic` builtins of builtins/proxy.ts (`%ProxyGet(p, key,
receiver)`, `%ProxySetOrThrow`, `%ProxyHas`, `%ProxyCall`, `%ProxyConstruct`, ...) with a
continuation that completes the op (D1085). The internal-method intrinsics
(OwnPropertyKeys, GetOwnPropertyDescriptorObject, DefineOwnPropertyFromRecord,
GetPrototypeOf, IsExtensible, SetIntegrityLevel, DeleteProperty, ...) check their first
argument: on a proxy they become a call of a builtin with the same parameters
(`REDIRECTS` in interp/run/proxy.rs), so every builtin written over them works on proxies
unchanged; they are tagged `@reentrant` now. Get/Set/HasProperty and the WithReceiver
forms also meet proxies further up the chain. The 13 traps follow 10.5.1-10.5.13 with
every invariant check; trap keys are strings or symbols (index keys, Numbers in the
intrinsics, become strings). Messages are qjs's ("revoked proxy", "proxy: inconsistent
get", ...). Leaves that need no user code stay in Rust: IsArray (now through proxies, a
TypeError for a revoked one), IsCallable/IsConstructor from the flags, the
Object.prototype.toString builtinTag, `typeof`, and `JsonStringifyFast` (Slow on a proxy,
or a proxy on the chain of a `toJSON` lookup or array hole). behavior.js runs our proxies
behind native shells against qjs's with logging handlers and invariant-breaking traps.

## D1451: `new` when reading new.target.prototype runs user code
date: 2026-11-01 · by: infra · issue: 0045
For a bytecode constructor whose new.target has a `prototype` getter or is (or inherits
from) a proxy, `construct` now calls `%OrdinaryConstructSlow(f, args, newTarget)`:
GetPrototypeFromConstructor in the builtin language, then the new reentrant
`%ConstructWithThis(f, this, args, newTarget)`, which enters f as a constructor with that
object (the tail of `construct_with`, now `construct_call`). This replaces "not supported
yet: getter for 'prototype'". GetPrototypeFromConstructor walks proxies for
GetFunctionRealm, so a revoked proxy as new.target is a TypeError.

## D1452: for-in through proxies is lazy, in qjs's trap order
date: 2026-11-01 · by: infra · issue: 0045
When the object or its prototype chain has a proxy, ForInStart makes a Record that
`%ForInStep` (builtins/proxy.ts) walks: an object's keys are read when the walk reaches
it ([[OwnPropertyKeys]], then [[GetOwnProperty]] per key for enumerability), each key
is checked again with [[GetOwnProperty]] just before it is visited (a deleted key is
skipped, as dquickts' test_for_in_proxy requires), and [[GetPrototypeOf]] runs when the
object's keys are used up. That is qjs's order for a proxy as the object itself. (For a
proxy further up the chain qjs reads its keys twice; we do not.) ForInNext on the Record
calls the step and runs again to push its result. Plain objects keep the eager Rust
snapshot. Other differences from qjs we keep because they are the spec's:
Object.isFrozen/isSealed ask IsExtensible before the keys (7.3.16).

## D1453: Primitive bases: [[Set]] through setters and proxies on the prototype chain
date: 2026-11-01 · by: infra · issue: 0045
`v.x = 1` / `v[k] = 1` for a primitive v (strict) used to be a TypeError outright. It is
OrdinarySet on the wrapper with v as the receiver: a setter on the prototype chain now
runs with `this` = v, and a proxy on the chain gets its `set` trap
(`%ProxySetOrThrow(p, key, v, receiver)`); anything else is still the TypeError
(test262 language/types/reference/put-value-prop-base-primitive.js).

## D1460: BigInt values are always heap cells, compared by value
date: 2026-11-02 · by: heap · issue: 0046
A BigInt is value tag 11 pointing at an immutable `BigInt` cell (kind 24): `LEN` u32
limbs, then little-endian `u32` limbs, normalized (no high zero limb; zero has no limbs
and is never negative), the sign in header flag `BIGINT_NEG`. One limb costs 16 bytes.
There is no inline small form: BigInt is rare in codemode, and a second representation
would double every BigInt path (and the reader's) for little gain; tag 7 stays free if
benchmarks ever want it. Equality is by value everywhere (`strict_equals`, `same_value`,
Map/Set keys hash the limbs), never by cell offset. The wrapper object is
`ObjectClass::BigInt` (30, slot 0 the primitive) with `%BigInt.prototype%` an ordinary
object (`Intrinsic::BigIntProto`). BigInt literals are constants in the pool.

## D1461: Arbitrary-precision arithmetic in `crate::bigint`, capped at 2^20 bits
date: 2026-11-02 · by: heap · issue: 0046
`crates/djs-core/src/bigint/` is heap-free sign-magnitude arithmetic on `Vec<u32>`
(u32 limbs so every product fits a u64, which wasm has natively): schoolbook
multiplication, Knuth D division, square-and-multiply powers (powers of two are shifts),
two's-complement bitwise ops, quadratic radix conversion (linear for power-of-two
radixes), and exact comparison with f64. Magnitudes are capped at `MAX_BITS` = 2^20 - 1
bits, QuickJS's limit, with its RangeErrors ("BigInt is too large to allocate"; "BigInt
is too large" for a power of two with a huge exponent). Size checks happen before the
work where the result size is known (mul, shl, pow), so `2n ** 10000000n` fails at once.
Operations run to completion and pay fuel afterwards (`mul_cost` = n*m/16 limb
products, linear passes n/16); the worst single operation (a 16k x 16k limb product or
quotient, or a decimal conversion of a 1 Mi-bit value) is a few hundred ms natively.
Karatsuba or divide-and-conquer radix conversion can come later if it ever matters.

## D1462: BigInt operators are a slow path beside `exec`
date: 2026-11-02 · by: heap · issue: 0046
The numeric operators check for a BigInt operand in `Exec::bigint_operands`
(interp/run/bigint.rs) right after `primitive_operands`, before the main match; the fast
paths stay Number-only. Mixed BigInt/Number arithmetic throws QuickJS's "cannot convert
bigint to number" after ToNumeric of both operands (so a symbol throws its own error
first), `>>>` "bigint operands are forbidden for >>>", unary `+` "bigint argument with
unary +". Relational operators, `==` (with Number and String: exact), ToBoolean and
ToString handle BigInt in `conv`. The builtins add `%ToBigInt` and `%ToNumeric`
(builtins/bigint.ts) and BigInt leaves at the end of the intrinsic fallback chain
(`json_regexp_intrinsic` -> `bigint_intrinsic`). `BigInt.prototype.toLocaleString`
ignores its arguments like node (qjs takes the locale as a radix and throws).
## D1400: Private names are private symbols in the object's own property table
date: 2026-11-02 · by: lexer · issue: 0040
Each private name of a class evaluation is a new symbol cell flagged `SYM_PRIVATE`
(heap/cells.rs flags; `SYM_PRIVATE_METHOD` marks methods and accessors for the brand
message, `SYM_PRIVATE_READONLY` methods and getter-only accessors, whose assignment
throws before the brand check, as in qjs). It is the value of a binding `#x` in the
class body scope, created by `private_symbol` at scope entry, so it exists before the
heritage and computed keys run. Private elements are ordinary entries of the object's
own property table under that key: `ordinary_own_keys` skips private symbols, so
[[OwnPropertyKeys]] and everything built on it (reflection, Object.assign, spread,
JSON, for-in, freeze/seal) never see them, and no user code can get hold of the
symbol. The private ops (`get_private_field` ...) read the own table only: never the
prototype chain, never a proxy trap (a proxy's private elements are its own, in the
proxy cell's table). Brand checks are presence checks: private methods are installed
per instance, as the spec has it. Cost: an atom table entry per private name per class
evaluation (atoms are not collected yet).

## D1401: Private elements need an extensible object (nonextensible-applies-to-private)
date: 2026-11-02 · by: lexer · issue: 0040
PrivateFieldAdd and PrivateMethodOrAccessorAdd throw `TypeError: object is not
extensible` on a non-extensible object, as test262's nonextensible-applies-to-private
tests require. qjs still adds them. A proxy is exempt (its heap cell is non-extensible;
its [[IsExtensible]] trap is not consulted). Freezing after the fact does not affect
private fields: they stay writable.

## D1402: Home objects are a closure slot after the captures
date: 2026-11-02 · by: lexer · issue: 0040
A function whose code (or an arrow in it) uses `super.x` has the `<home>` pseudo-binding
and `func_flags::HOME` (bit 6); `closure()` allocates one more object slot after the
captures, which `define_class`, `define_class_method[_elem]`, `define_method`/
`define_getter`/`define_setter` (and their `_elem` forms) and `set_home_object` set, and
`special_object HOME_OBJECT` reads. Functions without `super` pay nothing. `super.x`
pushes `this` and `home.[[GetPrototypeOf]]()` (`get_super`) before the key, in
MakeSuperPropertyReference order; the get/put ops take the base, not the home.

## D1403: Field initializers are two parked functions per class
date: 2026-11-02 · by: lexer · issue: 0040
All instance field initializers of a class compile into one `FieldInit` function, all
static ones (and the calls of the static blocks, each its own function) into another,
parsed by resuming the parked function state for each field. Each begins with a jump to
the private method installation emitted at its end, which jumps back, so
PrivateMethodOrAccessorAdd runs before the first field. The instance one is the hidden
binding `<fields N>` (`undefined` without instance elements) that a base constructor
calls at entry and `super()` calls after binding `this`; the static one runs once with
`this` = the class after the inner binding is initialized. Computed field keys are
converted at definition time into hidden bindings. One closure per class instead of one
per field; field initializer functions are never visible to user code.

## D1404: Derived constructors: `this` is a local, the result is checked after the body
date: 2026-11-02 · by: lexer · issue: 0040
`construct` gives a `KIND_DERIVED_CTOR` closure `this` = `undefined` and no prototype
lookup. Its `this` binding is the ordinary `this` pseudo-binding (a local or Box);
reads in the constructor and its arrows go through `check_this`, `super()` writes it
once (`check_this_uninit`). Returns store the value in an internal local and jump to an
epilogue after the body (`check_ctor_return`; `undefined` → the checked `this`), so the
result checks run outside the body's try statements, as [[Construct]] does them.
`super()` constructs `[[GetPrototypeOf]]` of the active function (`<active>`
pseudo-binding) with the frame's new.target. `new` of a non-constructor object now says
`<name> is not a constructor` (or `not a constructor` without a `name` string), like qjs.
## D1440: djs-difftest runs the shared prelude (supersedes D1160)
date: 2026-11-01 · by: infra · issue: 0044
The djs engine of djs-difftest now does what qjs_driver.js does, in-process: it evaluates
tests/corpus/prelude.js as a strict Script, calls its setup with a `host` object whose
`output` and `done` are the only host functions, evaluates the case as a Script
`(async (tools, console) => {<case>\n})` (an arrow: `this` is the global object, as in
pi and the qjs driver), passes it to `api.run`, and calls `api.stalled()` once nothing is
left to run. djs Scripts have no completion value, so both Scripts store their value in
a global (`__difftestSetup`, `__difftestCase`, on the first line, so line numbers are
kept) that the engine reads and deletes. The api object is held in a heap handle across
runs. Everything the model sees (console/text formatting, the return value, the
uncaught error, `exit`, `store`/`load`, the fake tools and ALL_TOOLS) is now the same JS
on both engines; the native formatting, the `<djs: ...>` slow-path markers and D1160's
known differences are gone. The fake tools stay JS in the prelude, resolved through
promise jobs exactly as on qjs, rather than async host functions: an async host call
settles only when the job queue is empty, which would order tool results differently
from the oracle. The prelude's `errorText` splits the stack with a small loop until
String.prototype.split lands with 0033 (marked in the prelude). The Sandbox (js/) needs
no change: it hands console arguments to the embedder's console as values, and pi's
own prelude formats in the VM.

## D1441: Object.prototype.__proto__, and Function.prototype.toString of proxies
date: 2026-11-01 · by: infra · issue: 0044
The Annex B accessor (B.2.2.1) is in builtins/object.ts: the getter is
`GetPrototypeOf(ToObject(this))`, the setter ignores non-object values and primitive
receivers and throws on undefined/null, with qjs's messages (also for
Object.setPrototypeOf now: "prototype is immutable" for Object.prototype, "proxy: bad
prototype" for a refusing proxy). Object literals keep their own `__proto__: v` form
(compiler) and `["__proto__"]` still defines an own property. Function.prototype.toString
of a callable proxy is `function <name>() { [native code] }` with the name read by
[[Get]] (a trap call), as qjs does.


## D1332: RegExp objects, suspendable matching and regex literals
date: 2026-11-01 · by: num · issue: 0033

**RegExp objects.** A RegExp object is `ObjectClass::RegExp` (25) with three slots:
- the compiled program, in a U32Array cell (an internal reference). 0011's program is
  pointer-free words, so the image carries it as is;
- the source string;
- the flag bits.

`lastIndex` is an ordinary own data property: writable, not enumerable, not
configurable, as in qjs. Leaf intrinsics live in builtins/regexp.rs and are dispatched
from interp/run/json_regexp.rs. The protocol is in builtins/regexp.ts, in spec order:
RegExpExec, flags, species, Symbol.match/matchAll/replace/search/split, GetSubstitution.
The six String methods are in builtins/string-regexp.ts. As in the current spec and in
qjs, only objects are asked for their Symbol methods. Messages are qjs's:
- "RegExp object expected";
- "regexp must have the 'g' flag";
- "cannot convert to object" for a null `this`;
- "'lastIndex' is read-only".

The RegExp String Iterator is class 26, with %RegExpStringIteratorPrototype%.
RegExp.escape follows qjs (lowercase hex).

**Matching.** `%RegExpExecRaw(r, s, lastIndex, state)` is RegExpBuiltinExec once
`lastIndex` has been read. It runs 0011's matcher in place on the flat string: a new
`Input::Utf16Le` reads two-byte heap strings without copying them. It gets the remaining
interpreter fuel, one unit per matcher step.

When the fuel runs out mid-match:
1. the ExecState words go into a new U32Array in the intrinsic's `state` argument slot
   (callers pass `undefined`);
2. `run` returns `OutOfFuel` with `pc` still on the `call_intrinsic`;
3. the next `run` executes the instruction again and resumes from that state.

Everything is in the arena, so a heap image taken while a match is suspended resumes
the same match. tests/regexp_fuel.rs checks this: plain, sliced and image-restored runs
give the same result with the same number of stops. `/(a*)*b/.test("a".repeat(30))`
simply runs out of fuel; qjs hangs on it.

A suspension costs O(backtrack stack): the state is serialized at every stop. With
realistic slices that is noise. With tiny slices on a pattern that keeps a deep stack,
it dominates (an e2e case with `(a|b)*` over 4,000 characters took minutes at fuel 997).

A backtrack stack over `ExecLimits::max_stack` (1M words) throws qjs's `InternalError:
out of memory in regexp execution`.

**Regex literals.** The compiler (expr.rs, `parse_regex_literal`) rescans `/` as a
regex token and compiles the pattern once for its early errors, reported at the literal
as qjs does. It then emits `undefined; push_const pattern; push_const flags;
call_intrinsic %RegExpCreate 3`, so every evaluation creates a fresh RegExp and
recompiles. Caching the program per literal is a possible optimization.

**JSON.parse errors** put qjs's `<input>:line:col` first line in `stack`.

## D1463: BigInt typed array elements: placeholders in `&self` reads, `Elem` writes
date: 2026-11-02 · by: heap · issue: 0046
A BigInt64Array/BigUint64Array element only becomes a value with an allocation, but the
heap's property reads (`get_own_property`, `find_property`, `lookup_property`,
`get_element_fast`, `ta_get`) are `&self`. They now return `Value::HOLE` as a
placeholder for a BigInt element (`get_element_fast` returns `None`, the generic path);
the places that hand element values to JS materialize it: the interpreter's
`lookup_object`, object spread (`copy_data_props`), `descriptor_object`
(getOwnPropertyDescriptor) and the `TypedArrayGet` leaf, through
`Heap::ta_get_alloc`/`get_own_property_alloc`/`find_property_alloc`/`materialize_value`.
New code that reads typed array elements for JS must use those. Writes take a
`typed::Elem` (a Number, or the 64 element bits): `Heap::element_of(kind, v)` converts a
primitive without user code or errors (a BigInt kind takes only BigInts, a Number kind no
BigInts), and `None` means the caller converts first: the interpreter calls `%ToBigInt`
instead of `%ToNumber` for a BigInt array, and the builtins use `ToElementValue`
(builtins/typedarray.ts). The leaves `TypedArraySet` and `DataViewSet` take a Value (a
Number or a BigInt by kind; anything else is ToNumber's/ToBigInt's TypeError), and
`DataViewGet` returns one. A species constructor with the other content type is a
TypeError (spec; qjs converts and throws later).

## D1480: TypeScript is a compile flag; the skipper ports dquickts' rules
date: 2026-11-03 · by: lexer · issue: 0048
`CompileOptions { typescript: bool, .. }` (and `CompileOptions::ts()`) enables the TS
grammar in the one parser; there is no separate TS front end. The type skipper and its
disambiguation (generic calls, arrows with return types, `as` precedence, the line
break rules of contextual keywords) port dquickts' `ts_*` functions one to one, and its
syntax error messages, so the oracle comparison stays exact. Speculation is lexer-only
(D1080) and its errors are message-free.

## D1481: Erased ranges are recorded by committing code, blanked like ts-blank-space
date: 2026-11-03 · by: lexer · issue: 0048
`Parser::erased` (on only for `compiler::strip_types`) collects `start..end` of every
erased type, recorded after the skipper committed, never during speculation. The
blanking writes one space per UTF-16 unit and keeps `\n` and `\r`, as ts-blank-space's
`getSpace` does, so `djs strip-types` can be compared byte for byte.

## D1510: Iterator helpers are slot objects with a hand-written state machine
date: 2026-11-03 · by: heap · issue: 0051
The objects of `Iterator.prototype.map/filter/take/drop/flatMap`, `Iterator.concat` and
`Iterator.from`'s wrapper are one heap class, `ObjectClass::IteratorHelper` (28), with 8
slots: the kind (0 wrapper, 1 map, 2 filter, 3 take, 4 drop, 5 flatMap, 6 concat, more
for the proposals), the generator state (0 suspended-start, 1 suspended-yield, 2
executing, 3 completed), the underlying iterator and its `next`, and per-kind state.
Instead of a generator closure, `%IteratorHelperPrototype%.next` runs one step of a
per-kind state machine in builtins/iterator.ts: a step yields (state 1) or returns,
which completes the helper; an exception completes it too. `return` at a yield closes
what the closure would (flatMap: the inner iterator, then the outer one). The builtins
read and write the slots with `%RecordGet`/`%RecordSet`, which now accept Records and
these slot objects (`is_slot_object` in interp/run/intrinsics.rs), after checking the
brand with the leaf `%ThisIteratorHelper(v, wrap)`. New leaf groups of 0051 live in
interp/run/late.rs, at the end of the intrinsic fallback chain.

Where qjs deviates from the spec, djs follows the spec and node: qjs closes the iterator
before reduce's "empty iterator" TypeError and closes an exhausted inner iterator of
flatMap (vm/tests.rs `iterator_helpers_follow_the_spec_where_qjs_differs`).
## D1490: pi codemode on djs: an adapter for pi's `startVm` contract, pi's prelude unmodified
date: 2026-11-01 · by: infra · issue: 0049
js/pi/vm.ts implements pi's VM-session contract (`startVm`/`VmSession`, the messages of pi's
protocol.ts) on djs, so pi swaps one module and keeps its launchers, host and prelude.
It works on heap handles (js/pi/engine.ts) rather than the Sandbox, because pi calls
the prelude's `settle`/`run`/`stalled` many times. The bridge is a sync host function,
not an async one: pi's prelude makes its own promises and settles them in `settle`, as on
QuickJS, and `vm_run` drains the job queue (D1370), so pi's `drain()` is one run plus
`stalled()`. The prelude and the script are compiled as async bodies (`return <expr>`),
since djs Scripts have no completion value; the script keeps pi's one-line prefix, so
line numbers match. An interrupt (`shouldInterrupt` after a fuel slice) is `vm_abort`
plus a `crash`. That is the same outcome as QuickJS's uncatchable interrupt.

## D1491: djs-wasm exports for embedders: file names, random seed, build hash
date: 2026-11-01 · by: infra · issue: 0049
Additive, ABI version unchanged: `vm_compile_file` (`vm_compile` with the file name
for stack traces; pi checks `codemode.js:2`, and the prelude drops frames by its file
name), `vm_set_random_seed(vm, lo, hi)` (a new VM's `Math.random` has a fixed default
seed, so a host needs it for real randomness and for seeded replays), and
`djs_build_hash()` (`BUILD_HASH`, which identifies the engine build in pi's deterministic
fingerprint).

## D1492: pi adapter: 10,000 fuel per interrupt poll
date: 2026-11-01 · by: infra · issue: 0049
pi's interrupt budgets count interrupt-handler polls. With 10,000 fuel between polls, a
tight loop polls about as often per second as pi's quickjs VM (34,000/s vs 32,000/s), so
a budget means the same CPU time. 100,000 made `while (true) {}` with pi's test budget run
for 6 s instead of 0.6 s. djs charges fuel for allocation and builtin work, which QuickJS
does not count, so such code stops much sooner on djs: a bound, not a regression. pi's
prelude takes about 120 polls (QuickJS: 4). Measured with work/scratch/0049/pollrate.ts;
numbers in docs/host.md.

## D1493: pi adapter: deterministic `Math.random` matches quickjs-wasi
date: 2026-11-01 · by: infra · issue: 0049
djs's `Math.random` is QuickJS's xorshift64* (D1314). pi's quickjs VM
ends up seeded with `seed + 1` (its first clock read) and one value drawn while the
context is created. The adapter therefore sets djs's state to one xorshift step after
`seed + 1`, and the same seed gives the same numbers on both engines; pi's pinned test
values pass. Non-deterministic runs seed from `crypto.getRandomValues`.

## D1494: pi adapter: quickjs-wasi's host globals in JS, before the prelude
date: 2026-11-01 · by: infra · issue: 0049
quickjs-wasi gives scripts `performance`, `queueMicrotask`, `atob` and `btoa`, and pi's
deterministic tests use `performance.now()`. The adapter defines them in a small JS
prelude (as `codemode-prelude.js`, so pi's prelude drops their stack frames) before pi's
prelude, whose lockdown then freezes them. `performance.now()` is `Date.now() - origin`
on the VM clock, with the origin set per run (deterministic: `startedAt`). Language
builtins that djs lacks (encodeURI, WeakRef, Iterator, ...) are not polyfilled. They are
engine follow-ups.

## D1495: pi adapter: reuse the VM image taken after the prelude
date: 2026-11-01 · by: infra · issue: 0049
Running pi's prelude (its lockdown) is about 90% of a djs start (9 of 10 ms). The
adapter caches the image taken right after the prelude, per compiled module, keyed by
everything the prelude reads (prelude text, tools, globals, store, heap limit; at most 16
entries, keys under 64 KB). A start with the same inputs restores the image (0.6 ms;
`Vm::from_bytes` verifies it) and then sets only the seed and `performance`'s origin.
A start then takes 0.5 ms, against 3.2 ms on quickjs-wasi. The handles of the prelude's
functions are part of the image (handles survive images, heap/handle.rs). Off with
`reuseImages: false`.

## D012: Development flow: per-agent job caps, iteration habits, worktree slots
date: 2026-11-01 · by: coordinator · issue: 0047
From devflow's measurements (work/issues/0047-devflow.md): the gate is not the main cost;
agents running unfiltered or release test262/difftest/qjs runs at full machine width are.
Adopted now: (1) the "Testing while iterating" section in AGENTS.md; (2) the repo's
`.cargo/config.toml` caps cargo at 6 jobs / 6 test threads, and the gate overrides to full
width; (3) the coordinator reuses one worktree per agent (`.worktrees/<agent>`) across
issues instead of a fresh worktree (and a cold target/) per issue. The test-side fixes
(difftest no-stress for the six timeout cases, parallel and fuel-capped e2e, the two slow
unit tests, the qjs result cache, runners honouring DJS_JOBS) are issue 0053. Rejected:
a shared CARGO_TARGET_DIR (artifact clobbering across branches), gate tiers, and release
builds in the gate.

## D013: Stack overflow is a RangeError
date: 2026-11-01 · by: coordinator · issue: 0049
pi's codemode runs on quickjs-wasi (quickjs-ng), which throws `RangeError: Maximum call
stack size exceeded`, as V8 does, and pi's tests expect it. dquickts (our oracle) throws
`InternalError: stack overflow`. djs follows pi and V8, since replacing pi's engine is the
goal. Difftest cases that hit stack overflow are marked as expected differences from the
oracle.

## D1500: Unicode property escapes, on by default
date: 2026-11-01 · by: num · issue: 0050
Supersedes D1112. `\p{..}` and `\P{..}` in `u` and `v` mode support:
- General_Category values and groups;
- Script and Script_Extensions;
- the 53 binary properties of ECMA-262's table, with exactly that table's aliases;
- the properties of strings in `v` mode.

`scripts/gen-unicode.py` generates `unicode/props.rs` from UCD 17 and the emoji 17 files.
Sets are varint ranges (gap, length); sequences are sorted and front-coded with zigzag
deltas; names are comma-separated alias lists. The tables:

| part | bytes |
| --- | ---: |
| General_Category leaves | 9.0K |
| Script + Script_Extensions | 5.8K |
| binary properties | 25.1K |
| properties of strings | 17.9K |

Total DATA is 57.8 KB. Derived sets are computed at lookup time (in
`unicode/property.rs`):
- Any and ASCII;
- Assigned, the complement of Cn;
- the gc groups;
- scx, which is sc minus the code points listed in ScriptExtensions.txt plus the
  script's own explicit list;
- RGI_Emoji, the union of the string properties.

Semantics follow qjs: the same error messages, `\p{Script}` is an unknown script, and
`\p{Lu=}` is `\p{Lu}`. For case-insensitive matching, `v` mode folds the set before
negating it and `u` mode after.

djs-wasm, release, stripped of custom sections:

| | bytes | gzip -9 |
| --- | ---: | ---: |
| without | 1,153,656 | 369,318 |
| with | 1,226,297 | 396,360 |
| difference | +72.6K (+6.3%) | +27K (+7.3%) |

Of the 72.6K, about 66K is data and 6.5K is code.

The `regexp-unicode-props` feature is **on by default**. Code written by models uses
`\p{L}`, `\p{Emoji}` and `\u` patterns, and qjs supports them, so a SyntaxError there
breaks real programs. 27 KB of gzip is small next to the base image and the interpreter.
Embedders that need the bytes turn the feature off and get D1112's SyntaxError. If size
gets tight, the properties of strings (18 KB, `v` mode only) are the obvious part to put
behind a second feature.

## D1501: Compact loops over one character (STAR)
date: 2026-11-01 · by: num · issue: 0050
A quantifier whose body matches exactly one character (CHAR, a class without strings,
`.`) compiles to a single `STAR` instruction:
- two registers hold the count and the start position;
- the greedy loop consumes as much as it can in one instruction (resumable, one fuel
  unit per character) and leaves one backtrack frame of kind `K_STAR`;
- each retry of that frame gives back one character (greedy) or takes one more (lazy).

So the backtrack stack stays flat. Before, there was one 4-word frame per iteration:
`/^\D+$/` on 2M units overflowed the 1M-word limit, and suspension copied the whole
stack at every stop.

Two refinements:
- **Follow set.** The compiler passes down the characters that can start what follows a
  node, in forward concatenations, when that cannot match empty (first_chars). `STAR`
  skips counts whose next character is not in that set without trying the
  continuation.
- **SKIP.** A pattern that starts with a greedy unbounded STAR, and has no
  backreferences, records the end of the run the loop consumed in a register written
  outside the undo log. When the attempt fails, the search loop skips to that end:
  every start inside the run would retry the same continuation positions.

The 0011 benchmark (release, utf16, best of two):

| pattern | before | after |
| --- | ---: | ---: |
| `[\w.+-]+@[\w-]+\.[\w.]+` | 6.4 ms | 1.5 ms |
| `\d+` | 1.0 ms | 0.6 ms |
| `\s+` | 3.0 ms | 2.1 ms |

Results:
- test262 built-ins/RegExp now passes completely: the three
  CharacterClassEscapes overflows are gone, and so are the 440 property-escape tests
  that overflowed on `^\p{..}+$`.
- A suspension now copies a small state.

## D1502: Regex literals compile once
date: 2026-11-01 · by: num · issue: 0050
The compiler already compiled every regex literal for its early errors. It now keeps the
program, in a U32Array cell held by an internal reference in the function's constant
pool, and emits `push_const program; push_const pattern; push_const flags;
call_intrinsic %RegExpCreate 3`. `%RegExpCreate` accepts the program in place of the
prototype argument, and RegExp objects share the immutable cell.

Every evaluation still creates a fresh RegExp object, as the spec requires. 100k
evaluations of `/a(b+)c/.exec(..)` went from 110 to 80 ms; qjs takes 10 ms, and the rest
is object creation and the exec protocol.

## D1503: Fast paths for unmodified RegExps
date: 2026-11-01 · by: num · issue: 0050
`%IsPristineRegExp(rx)` is true when nothing the RegExp protocol looks up can run user
code or differ from the original:
- rx is a RegExp whose only own property is a writable Number `lastIndex`, and its
  prototype is %RegExp.prototype%;
- that prototype still has the original `exec`, `flags` and eight flag getters,
  `constructor` and @@match (identity checks against the installed `@intrinsic`
  builtins, like D1351);
- %RegExp%'s @@species getter is the original.

With that:
- **`RegExp.prototype[@@split]`**, when the limit is undefined or a Number and the string
  is not empty, runs as the leaf `%RegExpSplitRaw`. The spec's sticky splitter, tried at
  every position, is equivalent to a search from that position (`regexp::exec_at` with
  `sticky` false).
- **`RegExp.prototype[@@replace]`** with a global pristine RegExp and a string
  replacement runs as `%RegExpReplaceRaw`: every match, GetSubstitution in Rust, and
  `lastIndex` 0 at the end.

Both are suspendable, like `%RegExpExecRaw` (D1332). Their progress record (an internal
ValueArray: the result array or the output so far, the positions, the matcher state) sits
in the `state` argument slot. Everything else takes the spec path in the builtin
language, and tests/corpus/builtins/regexp-fast-paths.js checks that every modification
is still observed like in qjs.

`exec` also passes a Number `lastIndex` straight to the leaf, which does ToLength itself.

Release build, 20k lines of text, 5 runs each:

| case | djs before | djs after | qjs |
| --- | ---: | ---: | ---: |
| `split(/[,;\s]+/)` | 3130 ms | 120 ms | 120 ms |
| `replace(/\s+/g, " ")` | 900 ms | 60 ms | 30 ms |
| `replace(/(\w+),/g, "$1;")` | 1110 ms | 160 ms | 80 ms |
## D1511: Array.fromAsync is a state machine resumed by builtin closures
date: 2026-11-03 · by: heap · issue: 0051
The spec runs Array.fromAsync's body as an async closure. The builtin language has no
`await`, so builtins/fromasync.ts keeps the closure's state in a Record and each Await
is `PromiseResolve` + `PerformPromiseThen` with two builtin closures (kinds 16
FromAsyncFulfilled and 17 FromAsyncRejected of `NewBuiltinClosure`) whose `this` is the
Record; a phase slot says where to resume. One reaction job per await, as the spec's
Await, so the tick order matches qjs (tests/data/e2e/array_from_async.js). The same
pattern fits other async builtins (AsyncDisposableStack).


## D1504: Undoing a log entry restores the slot's generation
date: 2026-11-01 · by: num · issue: 0050
The matcher's undo log entries are now `slot, old value, old slot_gen` triples (they were
pairs), and undoing an entry restores `slot_gen` too.

Without that, a write under a frame whose inner frames had logged the same slot and then
been popped was logged again. A `STAR` retry writes its count under the frame below
(D1501). When the continuation's own loop shares those registers, which happens because
the compiler reuses them after a loop, every retry added an entry.

pi's base64 check `/^[A-Za-z0-9+/]+={0,2}$/` on non-matching megabytes overflowed the
backtrack stack that way. Now 5 MB pass and fail in constant stack. Log entries cost one
word more.


## D1482: Enums and namespaces are closures; their members resolve after parsing
date: 2026-11-03 · by: lexer · issue: 0048
An enum or namespace block compiles to an arrow-like closure called with the object
(`N || (N = {})`), as tsc emits it. Its parameter has a name no source can write. A
declaration record per enum or namespace collects the members of all merged blocks, and
`resolve::classify` turns free names in the closures that are members into property
accesses on the parameter (`Target::Member`: a synthetic reference to the parameter,
classified like any other, so nested functions capture it). Doing it at resolution,
not while parsing, is what makes a block see members that a later merged block
declares, as tsc does. A type-only namespace's closure is discarded
(`Parser::discard_function`) and nothing is bound.

## D1483: const enums are folded at resolution
date: 2026-11-03 · by: lexer · issue: 0048
tsc inlines same-file const enum members even with isolatedModules, and dquickts does
too, so djs folds them: a binding that holds an enum or namespace knows its record
(`Var::ts_decl`), and the re-emit replaces `name_get CE; get_field A` (also
`N.CE.A`) with the member's constant when `CE` resolves to that binding. The enum
object is still created (preserveConstEnums); computed keys read it.

## D1484: Parameter properties follow useDefineForClassFields
date: 2026-11-03 · by: lexer · issue: 0048
`constructor(public x)` defines `this.x = undefined` before the class's other fields and
assigns `this.x = x` once the parameters are evaluated (base class: fields at entry,
assignments after the parameters; derived class: both right after `super()` returns,
around the field initializers), matching tsc's output with useDefineForClassFields and
dquickts. The names are found by a lookahead before the parameters are parsed.

## D1512: Disposable stacks: one slot class, resources as triples, async disposal as a state machine
date: 2026-11-03 · by: heap · issue: 0051
DisposableStack and AsyncDisposableStack are one heap class, `ObjectClass::DisposableStack`
(33): slot 0 whether it is async (the brand), 1 the state, 2 an internal Array of
[value, method, kind] triples. The spec's closures never reach user code, so they are
kinds instead of functions: 0 calls method with `this` = value, 1 calls method(value)
(`adopt`), 2 an async stack's resource that only has Symbol.dispose (GetDisposeMethod's
wrapper: the call's outcome is awaited, a throw becomes a rejection after the await), 3
an async stack's null/undefined resource (only forces the final await).
`disposeAsync` runs DisposeResources as a Record state machine resumed by builtin
closures 18/19, as Array.fromAsync does (D1511); the tick order matches qjs
(tests/data/e2e/disposable_stack.js). Symbol.dispose and Symbol.asyncDispose are
well-known symbol atoms (heap/atom.rs), so the `using` compiler work can use them.
test262: the explicit-resource-management feature stays later only for the `using`
syntax tests, listed by path in tests/test262.toml.
## D1540: D013 in practice: which overflows are RangeErrors; `djs-expect` in difftest
date: 2026-11-01 · by: infra · issue: 0054
`RangeError: Maximum call stack size exceeded` (`interp::STACK_OVERFLOW`) is thrown for
the interpreter's value stack limit and for JSON.stringify's nesting limit. V8 and
quickjs-ng report both as call stack overflows, and quickjs-wasi gives the same text for
both. Parse-time depth limits stay `SyntaxError: stack overflow` (the compiler, as qjs;
JSON.parse, which reports a position). Out of memory stays `InternalError`. The
difftest cases that hit the overflow keep qjs's `.expected` and declare the difference
with a new directive, `// difftest: djs-expect <oracle line> => <djs line>`. With djs,
`.expected` lines equal to the first text must read the second, and everything else is
still compared. The ratchet would have hidden the whole case.

## D1541: URI functions and escape/unescape as leaf intrinsics, with QuickJS's messages
date: 2026-11-01 · by: infra · issue: 0054
`encodeURI`, `decodeURI`, `encodeURIComponent`, `decodeURIComponent`, `escape` and
`unescape` are builtins (builtins/uri.ts) that ToString their argument and call four
leaves (`UriEncode`, `UriDecode`, `StrEscape`, `StrUnescape`; builtins/uri.rs). The leaves
throw URIErrors with QuickJS's messages (`expecting %`, `expecting hex digit`, `malformed
UTF-8`, `expecting surrogate pair`, `invalid character`) through a new
`ErrorKind::UriError`. dquickts and quickjs-wasi agree on all of them, and the leaves
follow QuickJS's decoder exactly, including which error wins for a truncated sequence.
escape/unescape are Annex B, which test262 skips for djs. pi's quickjs-wasi has them, so
djs does too. The leaves charge fuel per 8 code units. While there, Function.prototype
got its `length` (0) and `name` ("") (20.2.3). Their absence failed four URI tests that
delete a function's own `length`.

## D1542: Host display of errors never runs JS: accessor names fall back to the native name
date: 2026-11-01 · by: infra · issue: 0054
`error_summary` (`vm_display`, the CLI's "Uncaught", the Sandbox's error messages) reads
`name` and `message` as data properties only, because host formatting must not run user
code. After pi's lockdown, the native prototypes' `name`s are accessors, so errors
displayed as ": message". When `name` is not a data string, the summary now uses the
nearest native error prototype on the chain (TypeError, ..., Error). An own `name`, such as
one set by a subclass through the lockdown's setter, still wins.

## D1485: strip-types parses module syntax; djs still has no modules
date: 2026-11-03 · by: lexer · issue: 0048
The strip-types oracle's corpus (TypeScript's sources and conformance tests) is ES
modules, so `compiler::strip_types` sets `Parser::strip`, which accepts `import` and
`export` declarations, `import(...)` and `import.meta` (compiled to placeholders, never
run) and erases type-only imports, exports and specifiers as ts-blank-space does. Normal
compilation is unchanged: `import`/`export` stay "unsupported keyword" errors.


## D1513: WeakRef and FinalizationRegistry on ephemeron tables; cleanup jobs after a collection
date: 2026-11-03 · by: heap · issue: 0051
No collector changes: the weakness comes from D1382's EphemeronArray (a pair whose key
died is cleared). A WeakRef (`ObjectClass::WeakRef`, 31) holds a one-pair table
`[target, true]`; `deref` sees HOLE once the target died. A FinalizationRegistry
(`ObjectClass::FinalizationRegistry`, 32) keeps its cells in three parallel arrays: an
ephemeron table of targets, one of unregister tokens (both weak, as the spec wants),
and a strong ValueArray of held values (HOLE marks a free cell). A cell whose held value
is set and whose target is HOLE is dead. The realm lists the registries in an ephemeron
table too (`Intrinsic::Registries`), so a dead registry's callbacks never run (allowed).
heap/weakref.rs has the storage.

After every successful collection (the interpreter's safepoints and `Vm::collect`), the
interpreter queues one `%FinalizationRegistryCleanupJob` per live registry with dead
cells and no job queued yet, in registration order. `ExecOptions::finalization_after_gc`
(default on) is the host hook: off, nothing is queued until the host calls
`Vm::schedule_finalization_cleanup()`. The job calls the callback for each dead cell in
index order; an exception ends the job and is dropped (the spec leaves reporting to the
host; a finalizer must not make the run `Threw`), and the remaining dead cells wait for
the next scheduling. [[KeptAlive]] is `Intrinsic::KeptObjects`, filled by the WeakRef
constructor and `deref`, cleared whenever a job or the entry call finishes (`next_job`).
Everything iterates in index order, so collections stay deterministic (D1090), but GC
timing is observable through these objects: `--gc-stress` runs differ from plain runs
for scripts that let targets die. Tests keep targets reachable (e2e weakref.js) or
collect explicitly (vm/tests.rs).

## D014: Indirect eval and the Function constructors, no direct eval
date: 2026-11-01 · by: coordinator · issue: 0059
pi's suite expects `eval`/`Function` to exist, and the compiler ships in the VM anyway. djs
implements indirect (global) eval and the four dynamic function constructors. A call
`eval(x)` behaves as indirect eval. Direct eval (access to the caller's scope) stays a
non-goal (ARCH non-goals), and test262 tests that need it stay skipped.


## D1560: RegExp exec and test without protocol overhead
date: 2026-11-01 · by: num · issue: 0056
- `RegExp.prototype.exec` no longer calls `ToString` for a string argument, and it
  inlines RegExpBuiltinExec.
- RegExpExec (used by test, the @@ methods and matchAll) calls RegExpBuiltinExec directly
  when `exec` is the original builtin (`%IsOriginalRegExpExec`) and the receiver is a
  RegExp. That saves a call frame and the result check.
- `test` uses `%RegExpTestRaw`, which updates `lastIndex` but builds no match array.
- The leaves no longer intern names: the RegExp property and protocol names are
  well-known atoms (`index`, `input`, `groups`, `lastIndex`, `exec`, the flag names,
  `RegExp`).
- The program is copied out of its cell in one bulk read, and the backtrack stack starts
  with room for 64 words.

`%WellKnownSymbol` compares the name's bytes in place instead of building a String.

2M evaluations of a regex literal:

| | before | after |
| --- | ---: | ---: |
| `exec` | 1.91 s | 1.33 s |
| `test` | 2.33 s | 1.12 s |

## D1561: Bulk paths for apply, fromCodePoint and Math.max
date: 2026-11-01 · by: num · issue: 0056
- `Function.prototype.apply` passes a dense, hole-free Array straight to
  `%CallWithArray` (`%IsDenseArray`). Get on its elements runs no user code, so the
  copy in CreateListFromArrayLike is unobservable.
- `String.fromCodePoint` checks every argument in one leaf (`%AllCodePoints`): integral
  Numbers in range.
- CoerceNumbers (fromCharCode, Math.max/min) skips its loop when all arguments are
  Numbers (`%AllNumbers`).

Anything else takes the old per-element path, with its exceptions and order.

## D1562: A tight forward loop for STAR
date: 2026-11-01 · by: num · issue: 0056
A forward, case-sensitive STAR (D1501) consumes its run in one loop specialized per
input encoding. Classes start with the range that matched the previous character
(consecutive characters usually share a range), then fall back to the binary search.
Fuel accounting is unchanged: one unit per character after the first, stopping exactly
where the general loop would.

## D1563: Follow sets that accept the end of the input; disjoint loops
date: 2026-11-01 · by: num · issue: 0056
The follow set of a STAR (D1501) now records whether the continuation can also start at
the end of the input. A non-multiline `$` matches empty only there, and `start_cond`
tracks that, so `\s+$` and `[^=]+={0,2}$` get follow sets: before, the nullable tail
gave them none.

A greedy STAR whose matched characters can never start its continuation (body ∩ follow
= ∅, flag F_DISJOINT) tries the continuation only after the whole run, and fails without
giving characters back. pi's base64 check on non-matching data drops from O(n) retries
to one.

On the active test262 group (release, -j 6, two runs each):

| | before | after |
| --- | ---: | ---: |
| CPU (user) | 250–257 s | 90–91 s |
| wall | 61–66 s | 17–18 s |

The property-escape tests went from about 0.65 s to 0.2 s each. What remains in them is
the harness loop in the interpreter.
## D1600: Out-of-memory recovery: a collection at the handler and a reserve below the cap
date: 2026-11-03 · by: heap · issue: 0060
Allocation never collects (D1090), so before 0060 the garbage of the frames an OOM unwound
was still in the arena when the handler ran, and a handler that allocated (pi's prelude
formatting the error) could fail again: layout-dependent crashes. Now:

- **The error.** An allocation failure that reaches JS is the realm's preallocated
  `InternalError: out of memory` (`Intrinsic::OutOfMemoryError`, one object, never
  allocated at the throw, no `stack`), as in QuickJS. Rethrows are the same object.
- **A collection at the handler.** When `throw_value` lands that error in a handler, it
  collects right there (`safepoint_collect`), after the unwinding: the data of the
  frames it left is free before the handler's first allocation.
- **A reserve.** The heap holds back `min(cap / 32, 1 MiB)` below the memory cap
  (`Heap::hard_limit`; the arena's limit is the cap minus the reserve). Turning an
  allocation failure into the exception opens it (`Heap::open_memory_reserve`, a bit in
  `Roots::FLAGS`, so snapshots keep it), so handlers have room even when the data they
  still reference fills the heap, and the GC threshold drops so the next safepoint
  collects. The first collection that leaves twice the reserve free closes it again. An
  allocation failure while it is open is a plain OOM at the cap (the same error).
- **The contract.** To JS: an OOM is a catchable `InternalError: out of memory`; a
  script or host prelude that catches it and allocates less than the reserve until its
  frames have returned can continue (repeatedly: the reserve reopens after it closed).
  Uncaught, `run` returns `Status::Threw(that error)` like any exception; the heap stays
  valid and the VM usable. Embedder calls that allocate (`new_object`, `define`, ...)
  return `HeapError::OutOfMemory`. Usable memory for live data is the cap minus the
  reserve, so `Heap::with_limit` needs a 1/32 larger cap for the same contents.

Tests: vm/tests.rs sweeps limits from 1 to 12 MiB for five handler shapes (caller, same
frame, data still live, promise reaction, pi's prelude pattern), and js/test/pi-vm.test.ts
"the memory limit" expects the script's own error again, at several limits.

## D1601: The GC threshold is part of the heap image
date: 2026-11-03 · by: heap · issue: 0060
D1090 kept the collection threshold outside the arena and recomputed it on restore from
the image size, which includes garbage, so a VM restored from a snapshot collected at
other points than one that ran on, and the OOM point (which depends on how much garbage
is in the arena when the cap is reached) changed with snapshots. The threshold now lives
in `Roots::GC_THRESHOLD` and a restore keeps it (capped by what the restoring cap allows,
`min(stored, recomputed)`, so a smaller limit still collects in time). For a given script
and limit, the OOM point is the same with any fuel slicing and any snapshot/restore
points (vm/tests.rs `the_out_of_memory_point_is_deterministic`); `--gc-stress` is a
different policy and differs.
## D1590: CreateDynamicFunction parses one function and checks where its parts end
date: 2026-11-01 · by: infra · issue: 0059
The Function constructors build the spec's sourceText, `<prefix> anonymous(<P>\n) {\n<body>\n}`.
`compiler::compile_dynamic_function` (compiler/dynamic.rs) parses exactly one function
from it, with no binding of `anonymous` inside, at the top of a Script whose code returns
the closure. The spec parses P and the body separately. Two checks give the same result on the
joined text. The parameter list must end at the `)` the builtin inserted (`params_close`),
and nothing may follow the function. So `Function("){}; x(); (function(", "")`,
`Function("/*", "*/){")` and `Function("", "}{")` are SyntaxErrors. qjs evaluates the
joined source and would run the first one's injected code if it were not wrapped in
parentheses. The function's source text is the sourceText (Function.prototype.toString
matches qjs). ToString of the arguments happens in the builtin, in spec order. The four
constructors share one builtin (builtins/eval.ts), and new.target selects the prototype
through GetPrototypeFromConstructor. bgen now allows `new.target` in `@intrinsic`
builtins, because Rust installs the function-kind constructors as constructors (D1420).

## D1591: Dynamic code compiles in a leaf, runs as an ordinary call; fuel per code unit
date: 2026-11-01 · by: infra · issue: 0059
`CompileEval` and `CompileFunction` (builtins/eval.rs) only compile: they return a Script
closure, and the builtin calls it (`this` = the global object). Running the code is then
an ordinary JS call, so fuel, suspension, snapshots and GC need nothing new. A compile is
not interruptible, so the leaf charges it up front: 4 fuel per code unit of source,
roughly the cost of a few simple instructions per unit. The file name in stack traces is
`<input>`, as in qjs. Source is passed to the compiler as UTF-8. A lone surrogate in eval'd
source becomes U+FFFD; the compiler has no WTF-8 input. Eval code is a Script for now, so
it returns `undefined` and its top-level declarations go to the global scope. The TODO in
`compile_eval` is the one place where 0058's completion values and an eval goal (strict
eval's own environment) take over. The builtin needs no change.

## D1592: queueMicrotask is native; a throwing callback is reported like a rejection
date: 2026-11-01 · by: infra · issue: 0059
`queueMicrotask(callback)` (qjs and the HTML spec have it) queues the builtin job
`MicrotaskJob(callback)`, which calls the callback with no arguments and `this`
undefined. If the callback throws, qjs prints the error and drops the rest of its job queue.
Browsers report the error and go on. djs goes on and reports the error as an unhandled
promise rejection (a rejected promise with no handler), so hosts see it where they already
look (the Sandbox's `unhandledRejections`, the CLI's "Possibly unhandled promise
rejection"). The pi adapter's polyfill is gone. The adapter now removes WeakRef and
FinalizationRegistry in deterministic runs (as pi's quickjs VM leaves them out), and its
prelude images are keyed by the mode.

## D1593: test262's scan-eval group becomes direct-eval
date: 2026-11-01 · by: infra · issue: 0059
The skipped group is now `direct-eval`: `language/eval-code/direct` and tests whose
source has an `eval(` call (conservative: `obj.eval(` matches too, `(0, eval)(` does
not). 540 of the old group's 1,065 tests are active: indirect eval, built-ins/eval and the
Function constructors. 461 of them pass. The 79 failures are in the ratchet:
- 56 assume sloppy functions from `Function(...)`: duplicate parameters, `eval`/`arguments`
  as names, and `this` coerced to the global object (`Function("return this")()`, the
  harness's fnGlobalObject). djs is always strict.
- 20 need eval's completion value or strict eval's own environment (0058).
- 3 resizable typed array tests fail on unrelated gaps.
In the difftest corpus, the dquickts cases that use dynamic code run now. Four that read
the caller's locals through eval are `skip-djs` (direct eval). The rest pass or are in the
ratchet, waiting for completion values.

## D015: Sloppy `this` binding for Function-constructor functions
date: 2026-11-01 · by: coordinator · issue: 0059
Bundled libraries find the global object with `Function("return this")()`, which in an
always-strict engine returns undefined (and djs has no `global`/`self`). Functions created
by the four `Function` constructors whose body has no `"use strict"` directive therefore
get sloppy `this` binding (OrdinaryCallBindThis: undefined/null → the global object,
primitives → wrapper objects). Nothing else of sloppy mode: no implicit globals, no
mapped arguments, no `with`. Everything else stays strict.

## D1630: D015 as a template flag, bound on the slow call path
date: 2026-11-01 · by: infra · issue: 0063
compiler/dynamic.rs sets `func_flags::SLOPPY_THIS` on a Function-constructor function
whose body has no "use strict" directive. The body parser now records the directive in
`FuncState::use_strict`. `Exec::call` binds `this` for such a template before pushing the
frame: undefined and null become the global object, primitives their wrappers (which can
allocate, so it can fail like any call). The inline fast call path declines these templates
and falls back to `call`, so `push_frame` stays as it was. Nothing else of sloppy mode
changes. test262: 40 of the 56 sloppy failures from 0059 pass. The other 16 need sloppy
syntax (duplicate parameters, `eval`/`arguments` as names), which D015 leaves out.

## D1570: The wasm module links with --compress-relocations
date: 2026-11-03 · by: bloat · issue: 0057
wasm-ld writes relocated immediates (call targets, data addresses, `__stack_pointer`) as
padded 5-byte LEBs and nothing compacts them without wasm-opt: 52–58 KB, ~7% of the code
section (0052). djs-wasm's build.rs passes `--compress-relocations` to the cdylib link
when the target is wasm32 and the build has no debug info (wasm-ld rejects the flag
otherwise; the gate's debug build and `CARGO_PROFILE_WASM_DEBUG=1` line-table builds keep
the padded form). Same instructions, minimal LEBs: code 837,472 -> 779,856. A build.rs
rather than `.cargo/config.toml` rustflags, which cannot depend on the profile.

## D1571: The bytecode emit helpers are not inlined
date: 2026-11-03 · by: bloat · issue: 0057
`CodeBuf::begin`, `push_bytes` and `push_target` are `#[inline(never)]`. The generated
per-opcode emitters (`#[inline]`) call them from hundreds of compiler sites, and with
them inlined every site carried a `Vec::push`/`extend_from_slice` (length check, grow
call, store): code 779,856 -> 743,536 in wasm. Emitting is a small part of compile time
(lexing and parsing dominate), see 0057 for the compile-throughput check.

## D1572: The compiler's internal error is boxed
date: 2026-11-03 · by: bloat · issue: 0057
`PErr` is `struct PErr(Box<PErrKind>)` (`Syntax { pos, msg } | Oom`), built by the cold
`PErr::syntax` / `PErr::oom`. With the 16-byte enum inline, every `PResult<T>` was
returned through memory and each `?` (~800 in the compiler) copied it out and tested a
tag in memory; boxed, `PResult<()>` is one i32. Code 743,536 -> 712,176 in wasm. An
error costs one allocation, once per failed compile.

## D1573: Arena out-of-bounds panics carry no offsets in release builds
date: 2026-11-03 · by: bloat · issue: 0057
The heap field accessors are inlined everywhere, and each failed check called
`out_of_bounds(off, n, len)`: three arguments materialized at ~1,300 sites. Release
builds now call an argument-less `out_of_bounds_release()` ("arena access out of
bounds"); builds with debug_assertions keep the offsets in the message. The checks are
unchanged (no unsafe). Code 712,176 -> 699,863 in wasm.

## D1574: The interpreter's Fault is boxed
date: 2026-11-03 · by: bloat · issue: 0057
`Fault` is `struct Fault(Box<FaultKind>)` (`Throw(Value) | Error(ErrorKind, String) |
Heap(HeapError)`), built by the cold `Fault::throw` / `Fault::error` / `Fault::heap`.
`Result<(), Fault>` is then one i32 and `?` moves a pointer; `Result<Value, Fault>`
stays two words. Code 699,863 -> 686,908 in wasm. A fault costs one extra small
allocation; throw-heavy loops show no measurable change (0057).

## D1575: The wasm profile strips the custom sections
date: 2026-11-03 · by: bloat · issue: 0057
`[profile.wasm] strip = true`: the module comes out without its name section (175 KB)
and producers section, byte-identical to `wasm-strip` of the old output, so every
consumer of the `wasm` profile (scripts/build-wasm.sh, the Sandbox's default path,
djs-bench) gets the shipping module. `scripts/build-wasm.sh --keep-names` sets
`CARGO_PROFILE_WASM_STRIP=false` for profiling and scripts/wasm-funcs.py (a rebuild
when switching; `--strip` is accepted as a no-op). The size track adds `brotli -q 11`;
its `names` value is 0 from now on.

## D1580: `using` declarations keep a resource Record on the operand stack
date: 2026-11-03 · by: lexer · issue: 0058
A statement list (block, function body, catch block, codemode body) or a `for` head with
`using`/`await using` declarations compiles to a try/finally region that starts at the
first declaration: `%UsingCreate` pushes a Record (builtins/disposable.ts) that stays on
the operand stack under the statements, `%UsingAdd`/`%AwaitUsingAdd` append each value
(the 0051 AddDisposableValue checks and messages), and the region's finally subroutine
disposes, entered with `gosub` from the normal exit, the exception handler (which first
records the exception in the Record, so the subroutine rethrows it, wrapped in
SuppressedErrors) and every `break`/`continue`/`return`/generator return leaving it (a
`BlockKind::Finally` env). Sync disposal is one call (`%UsingDispose`); with an `await
using` in the list, the subroutine loops `%UsingDisposeStep` and a real `await` per
resource, so the awaits (needsAwait/hasAwaited, a sync disposal after a null `await
using`) tick exactly like qjs's. Alternatives: qjs's hidden (value, method) locals per
declaration with unrolled disposal (needs per-entry resets of the locals for loops, and
inlines the disposal at every jump), or one `await` of a disposeAsync-style promise
(extra ticks).

## D1581: Script and eval completion values; `Goal::Eval` for indirect eval
date: 2026-11-03 · by: lexer · issue: 0058
Script code (except the privileged prelude) and the new `Goal::Eval` return their
completion value: a hidden local updated by expression statements and reset by
`if`/loops/`switch`/`try`/`catch` (docs/compiler.md 5.8), as qjs's `eval_ret`.
`$262.evalScript` returns it with no change, and djs-difftest's temporary globals
(`__difftestSetup`, `__difftestCase`, D1440) are gone: the prelude and the case Script
are called for their values. `Goal::Eval` (`CompileOptions::eval`, `FuncKind::Eval`,
template kind KIND_SCRIPT) is strict eval code: top-level declarations are locals of
the call, free names are globals, `this` is the call's (the global object), `return`,
`new.target`, `super` and top-level `using` are SyntaxErrors, and `await` stays an
identifier as in any Script. builtins/eval.rs compiles indirect eval with it. Since djs
is always strict, test262's indirect-eval tests that need sloppy eval code (vars leaking
to the global object, global function declarations) now fail; they passed only because
eval code was a Script.

## D1582: `import()` without modules: compiled, rejected in a job
date: 2026-11-03 · by: lexer · issue: 0058
`import(specifier [, options])` compiles everywhere (with the import attributes argument
and a trailing comma) to `call_intrinsic %DynamicImport 2` (builtins/import.ts), which
follows EvaluateImportCall up to the host load: ToString of the specifier and the
`options`/`with` checks reject the promise with qjs's TypeErrors, and then, since djs has
no module loader, the promise is rejected one job later with qjs's no-loader error
(`ReferenceError: could not load module '<specifier>'`, the specifier not normalized).
`import.meta` stays a SyntaxError (module code only). pi needs the runtime rejection
(0049); a SyntaxError would fail the whole script. The test262 `dynamic-import` feature
stays skipped (modules): with this, 354 of its 682 tests would pass (syntax and
rejection-path tests), the rest need module fixtures.


## D1610: Append ropes: `s += x` writes into a growable buffer
date: 2026-11-01 · by: num · issue: 0061
Amends ARCH "Strings" (ropes for `+=`).

Before, a loop of `s += piece` built a left-deep rope with one node per append. The live
string was millions of nodes that every GC copied and traced, and flattening walked them
all: 924 MB RSS for 20M single-character appends.

**Layout.** No new cell kind:
- A Rope with flag `ROPE_APPEND` (4) is the first `LEN` code units of the flat cell in
  its `LEFT`. `RIGHT` is null.
- That cell is an *append buffer*: a Str8/Str16 with flag `STR_APPEND_BUF` (1), spare
  capacity past `LEN` (its used length), and zeroed slack.

**Append.** `concat(a, b)` writes b into a's buffer in place, then returns a new
24-byte append rope, when all of these hold:
- a is an append rope;
- the buffer is not frozen;
- a's `LEN` equals the buffer's used length (no other string sees the units past it);
- there is room;
- the width fits (a one-byte buffer takes only one-byte pieces).

Otherwise:
- An append chain gets a new buffer, about twice the length, with the string copied
  (doubling, so amortized O(1) per unit). A chain means a is already an append rope, or
  a concatenation rope of at least 256 units whose right part is short, and b is not
  longer than a.
- A two-byte piece moves the chain to a Str16 buffer.
- Everything else (short expression temporaries like `"id " + i + "\n"`, big + big)
  stays an O(1) rope or a short copy.

**Invariants** (verify() enforces the first):
- **(a)** Only an append rope's `LEFT` may reference an unfrozen buffer. No Value and no
  other ref does.
- **(b)** Only the rope at the buffer's end writes into it. An older, shorter rope over
  the same buffer that is appended to copies, and keeps its value.
- **(c)** A frozen buffer is never written again. `flatten()` of an append rope that
  covers its whole buffer freezes the buffer (clears the flag) and redirects the rope to
  it, so the buffer may then be referenced directly. A shorter rope flattens to a copy.

**Reads.** `str_view`, `str_view_flat`, the chunk iterator and the matcher read the
prefix in place, without freezing.

**GC.** Unchanged:
- Buffers are copied with their capacity; walk.rs already allows slack.
- Dead intermediate append ropes are garbage and are not traced.
- The FLATTENED short-circuit applies to frozen buffers.
- Collecting twice gives identical bytes.

**Determinism.** Buffers, capacities and slack depend only on the sequence of
operations; a test builds the same strings in two heaps and compares the images.

**Images and readers.** Images round-trip, and a restored buffer keeps appending in
place. js/src/reader.ts slices `LEFT` to the rope's `LEN` (docs/wire.md;
layout.generated.ts exports the new flags). Images from before this change contain no
append ropes, so they read the same.

**Numbers** (release djs-cli; min of 3 runs; user+sys):

| case | before | after | qjs |
| --- | ---: | ---: | ---: |
| 10M `s += "x"` in a function | 1.13 s, 574 MB RSS | 0.93 s, 150 MB | 0.6 s |
| 20M at the top level | 924 MB RSS | about 180-290 MB | 32 MB |
| 300k template literals `html += \`<li…>\`` | 0.21 s, 172 MB | 0.17 s, 107 MB | 0.15 s |
| engine/djs/string-append | 65 ms | 60 ms (qjs 35 ms) | |

The rest of the engine track moves within the ±10% that a one-atom change also produces
in code layout (checked with a probe build).

## D1611: JSON.parse source text access
date: 2026-11-01 · by: num · issue: 0061
`json-parse-with-source`, as of the test262 commit, is promoted to active.

**Parse with a reviver.** `%JsonParse(text, true)` parses as before, then walks the
source text next to the parsed values (builtins/json.rs `source_records`, an explicit
stack). It builds the JSON Parse Record tree as internal 4-element Arrays
`[value, source, elements, entries]`:
- `source` is the exact source text of a primitive;
- `elements` is an Array of the element records;
- `entries` is a null-prototype object from key to record. The last of duplicate keys
  wins, as the proposal says; qjs instead gives no source there.

InternalizeJSONProperty (builtins/json.ts) passes each record down. While SameValue of
the record's value and the current value holds, it creates the reviver's third argument,
a plain object with `source` for primitives and empty for objects and arrays, and uses
the element and entry records. Values replaced or added by the reviver get no source.
Without a reviver nothing changes, and there are no records.

**JSON.rawJSON(text)** returns a frozen, null-prototype `ObjectClass::RawJSON` (29)
object with an enumerable `rawJSON` string. `text` must be a JSON primitive without
leading or trailing tab, LF, CR or space; otherwise it throws qjs's SyntaxError "invalid
rawJSON string".

**JSON.isRawJSON** checks the class. **JSON.stringify** emits a RawJSON object's text
verbatim. The leaf fast path sees an unknown class and falls back to the full
algorithm, which does it.

## D1660: pi adapter: the prelude is not polled; the fingerprint hashes the host globals
date: 2026-11-03 · by: determinism · issue: 0066
pi's launchers count `shouldInterrupt` polls over a whole execution, and a start that
reuses a prelude image (D1495) skips the host globals and the prelude. While the adapter
polled that work too, a cold start used about 136 polls more than a warm one, so a
deterministic replay after a restart (cold) could time out where the original run (warm)
did not (0055). The adapter now does not poll while the host globals and pi's prelude
run (trusted, bounded code, about 10 ms). Polling starts with the per-run work that both
paths share (the performance origin, the script), and cold, cache-filling and warm
starts count the same polls (js/test/pi-vm.test.ts). A stop request during the prelude
waits until it has finished. The deterministic fingerprint adds `host <fnv1a of
HOST_GLOBALS>` after the prelude hash, because the adapter's own globals are part of
what a script observes; `DETERMINISTIC_VERSION` stays for other changes to the adapter.

## D1640: The REPL's line editor is hand-rolled, on `stty`
date: 2026-11-01 · by: repl · issue: 0064
`djs repl` edits lines itself (crates/djs-cli/src/repl/term.rs, about 300 lines) instead
of using rustyline or similar. A crate would bring a dependency tree (nix, unicode
tables, ...) and would still not solve the hard part: stopping a running input on
Ctrl-C needs a signal handler, which std does not have. Instead the terminal goes to
non-canonical mode without echo or signal keys (`stty -icanon -echo -isig -iexten`, the
saved settings restored on drop), so Ctrl-C is a byte. One thread reads stdin into a
channel; the editor reads keys from it, and the run loop polls it between fuel slices
(djs can always stop between slices) and calls `vm.abort()`. No `unsafe`, no libc. Without
a terminal, or without `stty`, the REPL reads plain lines (line mode, used by the tests).

## D1641: REPL inputs: expressions, Scripts, and a rewrite for top-level await
date: 2026-11-01 · by: repl · issue: 0064
Each input is compiled in this order (crates/djs-cli/src/repl/input.rs, docs/cli.md):
1. As an expression, if it starts with `{` (node prints `{a: 1}` as an object): the async
   body `return [(input\n)];`, whose value is printed. The one-element array keeps a
   promise value from being adopted.
2. As a Script (D004), so declarations persist as globals. Its completion value (0058,
   D1581) is printed, except `undefined` from an input that starts with a declaration.
   Then as an expression (step 1's wrapper), for `await` expressions. Before 0058 landed
   every expression took step 1 (`SCRIPT_COMPLETION_VALUES` in input.rs was the switch).
3. If it contains `await`: rewritten into an async body whose top-level declarations
   assign globals, which a Script declares first: `let`/`const`/`var` keywords become
   `0,` padded to the same width (so `let {a} = o` is the destructuring assignment
   `0,  {a} = o`), `class C` becomes `C = class C …;`, and a function declaration is
   followed by `globalThis.f = f`. Assigning globals (instead of copying the body's
   locals out at the end, as the alternative was) keeps closures made in the input
   bound to the globals, as node's REPL does. Gaps: `const` becomes `let`, statements with
   `await` print no value, a throw leaves the declared names `undefined`.
4. Otherwise, if a compile error is at the end of the input (or is an unterminated
   template or comment), the REPL reads another line; else it reports the error.
The top-level declarations are found with djs's own lexer, a bracket stack, and the
previous token to tell a regex from a division.

## D1642: `Vm::run_metered` and `Vm::backtrace`
date: 2026-11-01 · by: repl · issue: 0064
Two additive accessors for debuggers: `run_metered(fuel) -> (Status, used)` (the REPL's
fuel accounting and limit; `used` can exceed `fuel` by the cost of the last instruction),
and `backtrace()`, the `Error.stack`-style trace of a suspended call (out of fuel or
parked), empty without frames. They need two one-line `pub(crate)` getters on `Exec`
(`fuel_left`, `pc`); `Vm::run` is unchanged.

## D1620: Inline caches: verified table indexes in a per-template array
date: 2026-11-03 · by: heap · issue: 0062
Property and global access sites (`get_field`, `get_method`, `put_field`, `get_global`,
`get_global_typeof`, `put_global`) carry a `Cache` immediate (u16), numbered in code
order by `alloc_template` (the emitters write 0; `NO_CACHE` = u16::MAX past 65,535
sites). `FuncTemplate::IC_COUNT` is the number of sites, `FuncTemplate::IC` a U32Array
of three words per site, allocated when the first slot is filled (so the base image has
none). A slot remembers the **index of the entry in a property table** where the property
was found; a hit re-reads that entry and checks the key and that it is a (writable) data
property (`Heap::data_at`, `set_data_at`), so a stale slot is a miss and caches never
change semantics. Slots hold no arena offsets that outlive a collection (indexes and
epochs only), so the GC ignores them, and a VM restored from an image with warm caches
behaves exactly like one with cold caches (vm/tests.rs
`warm_and_cold_inline_caches_behave_the_same`). Global slots point into the global
lexical record or the global object; for the latter, the slot also holds the realm's
`GlobalLexEpoch`, bumped by every new global lexical binding, which could shadow it.
docs/objects.md has the measurements and the rest of the plan (own properties, prototype
methods, then shapes as a separate issue).

Benchmarking note: release builds with the default 16 codegen units vary by ±15% on
unrelated benchmarks between builds of nearly the same source (and of the same source in
another directory), from code layout. Compare builds made with
`CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 CARGO_PROFILE_RELEASE_LTO=fat`, from the same
directory.

## D1760: Unicode normalization, on by default
date: 2026-11-01 · by: num · issue: 0076
Supersedes D1342's "normalize has no tables" and its code-point-only `localeCompare`.
`String.prototype.normalize` implements NFC, NFD, NFKC and NFKD (UAX #15) from UCD 17 in
`unicode/normalize.rs`: full decomposition of the single-level UnicodeData mappings
(recursively), canonical ordering (a stable insertion sort of each run of nonzero
classes), and canonical composition with Full_Composition_Exclusion; Hangul syllables
are decomposed and composed algorithmically. Lone surrogates pass through unchanged.
`localeCompare` NFC-normalizes both strings before its code-point compare, as qjs does.

`scripts/gen-unicode.py` generates `unicode/norm_tables.rs`:

| part | bytes |
| --- | ---: |
| decompositions: a varint stream + an index every 32 entries | 21.2K |
| primary composites: (first, second index) keys + i16 deltas | 6.1K |
| Canonical_Combining_Class runs | 2.0K |
| NFC/NFD/NFKC/NFKD quick-check No/Maybe ranges | 4.3K |

The stream codes each mapping's first code point as a delta from the previous entry's
(neighbouring compatibility characters map to neighbouring targets), and runs of
one-to-one mappings onto consecutive code points (the mathematical alphanumerics, the
fullwidth forms) as one entry: 32.1K → 20.1K raw. Lookup is a binary search in the index
and a scan of at most 32 entries.

The fast paths: a string whose code units are all below U+0300 (NFC) or U+00A0 (other
forms) is returned as is after an in-place scan, without copying; otherwise the UAX #15
quick check (ordering of classes, No/Maybe lists) returns the input for YES, and a
normalization whose result equals the input also returns the original string. 1 MB of
mixed Latin/Hangul through NFD, NFC and NFKC takes about the time qjs takes.

djs-wasm, `wasm` profile (stripped), on the 0076 branch:

| | bytes | gzip -9 |
| --- | ---: | ---: |
| without | 1,261,895 | 435,191 |
| with | 1,300,133 | 455,727 |
| difference | +38.2K (+3.0%) | +20.5K (+4.7%) |

The `unicode-normalization` feature is **on by default**, for D1500's reasons: qjs
normalizes, codemode programs normalize and compare user and tool text, and a TypeError
for any accented input breaks them. Without the feature, `normalize` keeps D1342's
behavior (strings below the limits above come back, others throw "not supported yet")
and `localeCompare` compares code points.

Tests: a unit test runs every line of UCD's NormalizationTest.txt (all five columns in
all four forms, plus the quick check never answering YES for a string that changes)
and checks that every code point not listed in part 1 is invariant; it reads the file
from `vendor/unicode/17.0.0/` of the primary checkout, which `scripts/gen-unicode.py`
fetches. qjs and djs both pass all 400,680 of these checks (a scratch script), and
`tests/corpus/builtins/normalize.js` compares them on selected cases.

## D1670: Async host results are macrotasks: the host inbox
date: 2026-11-03 · by: determinism · issue: 0067
Amends D1374 and D1370. Before, `resolve_host_call` on an async call queued its
`%HostCallSettle` job at once, in any VM state, so a result delivered between two fuel
slices landed in the middle of the microtask queue. The Sandbox passes results back after
every slice, so its output depended on `fuelSlice` (0055). Now the result goes into the
host inbox, `Interp::INBOX`: a ValueArray of `[promise, ok, value, now]` entries from
`INBOX_HEAD`, part of the heap and of images. When the job queue runs empty (`next_job`),
the run loop takes the oldest entry, reserves a job slot first (out of memory leaves the
entry where it is), sets the clock if the entry has one, queues the settle job and
drains the queue before it takes the next one. `Idle` means: no jobs, an empty inbox,
and async calls pending. A script's behavior then depends only on the order of the
results, as with I/O callbacks in an event loop and as pi's prelude does by hand
(settle, drain, next).

Unhandled rejections: every time the queue runs empty is a checkpoint
(`rejection_checkpoint`). `Interp::REPORTED` counts the leading entries of `REJECTIONS`
that were unhandled at a checkpoint. A JS handler added later does not withdraw them,
so taking the reports after several results gives the same list as taking them after
each one. The embedder's `mark_promise_handled` (`vm_promise_state`, D1423) still
withdraws them.

`Vm::resolve_host_call_at(id, result, now_ms)` (`vm_resolve_host_call_at`) carries the
clock `Date` reads from when the result is applied (for the parked sync call: right
away). A host gets a virtual clock per result without stepping the VM between results.
`Vm::abort` drops the inbox with the jobs.

## D1750: The build hash covers features and libm; canonical f32/f16 NaN
date: 2026-11-03 · by: num · issue: 0075
From 0055's G8/G9 (P9). `BUILD_HASH` (D1330) changes whenever behavior can change.

**Build identity.** `crates/djs-core/build.rs` adds the following to `SOURCE_HASH`, after the
sources and the prelude:
- the enabled cargo features of djs-core, sorted, without `default`. Without
  `regexp-unicode-props`, `\p{..}` is a SyntaxError; without `unicode-normalization`,
  `normalize` throws;
- the `libm` version, because Math results come from it.

`libm` is now pinned with `=` in djs-core's Cargo.toml, so the version that build.rs reads
there is the one that gets compiled, even when another workspace builds djs-core. A bump
is a deliberate edit, and it changes the hash. The inputs are also exported as
`vm::BUILD_INPUTS` (e.g. `["feature regexp-unicode-props", "feature
unicode-normalization", "libm 0.2.16"]`). A test checks that list against `cfg!(feature)`
and against Cargo.lock.

**Float NaN.** Converting f64 to f32 with `as f32` compiles to `f32.demote_f64` on wasm,
and the wasm spec leaves the NaN bits of that instruction nondeterministic (likewise
`promote`). The bits are observable through a Uint8Array over a Float32Array's buffer.
Three places used to rely on that conversion: Float32 elements (typed arrays and DataView
`setFloat32`), `Math.fround` and Float32 decode. They now go through
`heap::typed::{f64_to_f32_bits, f32_bits_to_f64}`, which handle NaN explicitly:
- writing a Float32 element gives `0x7fc00000`, as in qjs and V8;
- reading one gives `f64::NAN`.

Float16 NaN no longer takes the sign of its input and is always `0x7e00`, which is what
V8 writes. qjs writes `0x7c01`, but the spec leaves the bits implementation-defined, and
djs already wrote `0x7e00` for every canonical NaN. Observable behavior is unchanged on
every platform measured in 0055. The change makes it hold by construction on wasm too.

## D1650: Decorators after qjs, the runtime in builtins/decorators.ts
date: 2026-11-03 · by: lexer · issue: 0065
TC39 decorators (2023-11) and `accessor` compile like qjs's implementation
(docs/compiler.md 7.11): the same application order, context objects (property order
kind, access, static, private, name, addInitializer, metadata), messages, metadata
inheritance and class replacement semantics (the static initializer and the static
extra initializers run with `this` = the new class; static private methods are
installed on it, as the static initializer installs them). The work is in builtins:
`%DecoratorStart`, `%ApplyDecorators`, `%DecoratorRunInits`, `%DecoratorRunExtras`, and
four builtin-closure kinds (20 addInitializer, 21-23 the public `access.get/set/has`),
so the interpreter only gained the closure kinds and the well-known symbol
`Symbol.metadata` (heap atom `SYM_METADATA`). Private access functions and the
auto-accessor getter/setter are synthesized bytecode functions instead of intrinsics, so
no private-field intrinsic is needed. Hidden decorator bindings are named `<dec>N` in
qjs's allocation order, since an auto-accessor's storage name appears in qjs's brand
check message. Known divergence: qjs rejects `@dec abstract class` in TypeScript; djs
accepts it.

## D1720: Determinism checks in the gate: heap images across schedules and builds
date: 2026-11-03 · by: determinism · issue: 0072
Three checks, all hard failures in the gate, all comparing heap images *after a final
collection*. The copying GC lays a heap out as a function of its object graph (D1090) and
collects at the same points under any slicing and restore (D1601), so equal graphs give
equal bytes, and an image difference means the run built a different graph.
- **e2e:** the plain and suspension modes (fuel 1, then 997 per run, restoring the image
  at every stop) must leave identical images.
- **difftest `--determinism`:** suspended variants at 97 and 1009 fuel (restoring at
  every stop) must leave the plain run's image. The plain run and its variants share one
  clock reading per case (`Job::now`), since the clock is heap data.
- **native vs wasm:** `djs-abidrive` (djs-difftest, natively) and
  `scripts/determinism/abidrive.mjs` (the gate's wasm build) make the same export calls
  for four bodies at two slices. Build hash, raw and collected image hashes, stop counts
  (fuel) and results must match.

GC stress is excluded from the image comparisons. It is another collection policy: weak
tables lose dead entries at other times and compact differently (e2e `collections.js`
differs only through its WeakMap). Raw images (without the final collection) are not
compared yet, because suspended intrinsics allocate progress records only when a slice
ends inside them, and `run` collects at its start (0071). Part 2 of 0072 adds them after
0071. Cost: about +25 s wall for difftest at -j 6 (one more variant), 7 s for the
native/wasm check.
## D1621: Prototype property caches: a receiver-prototype offset, an epoch, and GC clearing
date: 2026-11-03 · by: heap · issue: 0062
A `get_field`/`get_method` slot (D1620) may also remember a data property found on the
prototype chain: the depth of the holder, its entry index, the offset of the receiver's
prototype (an object's, or the realm's prototype of a primitive), and
`Roots::PROTO_EPOCH`. A hit checks the epoch, that the receiver is not a proxy or a typed
array and has no own `key` (a string's `length`/indexes never hit), that its prototype is
the cached one, walks to the holder and re-checks the key at the index there. Filling
marks every object from the receiver's prototype to the holder `OBJ_IS_PROTO` (flag 64,
never cleared); adding a key to such an object (`table_insert`) or changing its prototype
bumps the epoch. The receiver's own changes are covered by the per-hit own check, the
holder's by the key check. These are the only slots with arena offsets (word 0 bit
`IC_OFFSET_SLOT`): the collector empties them when it copies a template, rather than
bumping the epoch, so a collection of a collected heap stays byte-identical (D1090). Only
data properties are cached; getters take the slow path.

## D1631: Native frames are ordinary frame records with a header bit
date: 2026-11-03 · by: infra · issue: 0063
A `@native` builtin runs in a frame with the layout of a bytecode frame (frame.rs):
callee (the builtin's own Function object), `this`, arguments padded to the parameter
count, the 6-slot header with `FLAG_NATIVE` (bit 5 of `flags`), then the slots bgen
assigned (locals and temporaries, all Values; i32/u32/f64/bool as number/bool Values)
and an operand area for outgoing calls. `pc` is the state; a callee's `ret_pc` is the
resume state. Nothing in the frame is a pointer, so the GC (which traces the stack
array), heap images, the suspension stress mode, stack traces and `Vm::backtrace`
(`at map (native)`, as for bytecode builtins) need no change. The prelude marks the
builtins with `%MarkNative(fn, index)` (bgen emits it): `func_flags::NATIVE` plus the
index in `FuncTemplate::NATIVE` (the former RESERVED2); a name that does not match the
generated table fails the prelude, so the base image build fails. Dispatch:
`Exec::call` pushes a native frame when the template is NATIVE and
`ExecOptions::native_builtins` is on (the default); `fast_call_at` declines those
templates through the flags word it already reads, so JS→JS calls cost nothing more.
`load_frame` reads the header bit and sets `code_len` to 0, so `fast_step` falls through
and `step`'s cold out-of-range branch calls `native_step`. The header bit, not the
option, decides what a running frame is: changing the option between runs only affects
new calls. Exceptions: `native_step` turns faults into thrown values in its own frame;
`throw_value` asks the generated handler table (state → catch state, exception slot);
the key of a native caller after unwinding is its resume state (`call_site_key`), since
a resume state is in the same `try` region as its call.

## D1632: The bgen Rust backend: states are basic blocks, calls go through call_intrinsic
date: 2026-11-03 · by: infra · issue: 0063
`crates/djs-bgen/src/emit_rust.rs` writes `crates/djs-core/src/builtins/gen/natives.rs`
(checked in, `scripts/bgen.sh --check` covers it). A builtin is lowered to basic blocks,
one match arm (state) each; blocks end at branches, back-edges and resume points.
Within an expression containing a resume point, call results are spilled to temporary
slots in evaluation order and `&&`/`||`/`?:` around resume points become branches, so
Rust locals never live across a state boundary. Every intrinsic call goes through the
interpreter's `call_intrinsic` on the operand area (one implementation for both
backends); a reentrant one either completes inline (continue at the resume state in the
same Rust loop) or pushes a frame and returns to the run loop. Inline fast paths
(`interp/run/native.rs`) mirror `call_intrinsic`'s and `fast_intrinsic`'s exactly: dense
Array elements for Get/HasProperty/CreateDataPropertyOrThrow/Set, an Array's length for
LengthOfArrayLike, `length` set to itself, a handful of leaf type tests. Callees that
are builtins: leaf ones become plain Rust functions; reentrant ones are inlined when they
are small internal helpers (at most 8 statements, no installs, so never in a stack
trace, no `this`, rest parameter, `try` or `new.target`, depth ≤ 3, not recursive),
otherwise they must be `@intrinsic` (bgen says so) and are called through
`call_installed` (a native callee gets its own native frame). A rest parameter only read
by `LengthOfArrayLike(rest)` and `Get(rest, i)` is not materialized (push). Numbers:
i32/u32 `+ - *` and unary minus are checked (D1171: an overflow is an InternalError);
values are made with `conv::number`, so both backends produce the same Values. Known
gaps: `try`/`finally` and `@constructor` in `@native` builtins are bgen errors; an i32
`-0` (JS `-x` for x = 0) is 0 in Rust (only observable if passed to user code).

## D1633: Fuel per transition; the backend is part of the engine's identity
date: 2026-11-03 · by: infra · issue: 0063
Native code charges 4 fuel per loop back-edge and 2 per call it makes (bytecode charges
per instruction), and back-edges are safepoints, so a long `map` suspends and collects
mid-loop like bytecode. Fuel counts therefore differ between the backends (results do
not), so the backend choice is part of what determines a run: the wasm ABI has no switch
(always native), and `BUILD_HASH` covers the generated code (it is a djs-core source)
and the prelude's `%MarkNative` list, so `djs_build_hash` identifies it. An embedder
that turns `native_builtins` off must include that in its own fingerprint.

## D1634: Code size: the native API is out of line in wasm
date: 2026-11-03 · by: infra · issue: 0063
The generated code calls small `Exec::n_*` helpers for every slot access and call.
Inlined (as on hosts) they cost 180 KB of wasm for the first 35 natives; with
`#[cfg_attr(target_arch = "wasm32", inline(never))]` on the hot helpers it is 71 KB of
generated code plus 7 KB of runtime: the code section grows by 80 KB (+9%) over the
merge base. `scripts/native-sizes.sh` prints the size per native function; a new
`@native` needs a benchmark that justifies its size (ARCH "Rust backend").

## D1635: Both backends are tested
date: 2026-11-03 · by: infra · issue: 0063
`--backend bytecode|native` in `djs run`, djs-difftest and djs-test262; the engine
track runs `djs` and `djs-bytecode`. The gate runs the corpus natively in all stress
modes and once more with the bytecode backend (plain); the test262 active group with the
bytecode backend is a periodic check (TESTING.md "Builtin backends"), at 0063 both pass
37,711/37,912.

## D1680: Deterministic mode: weak references collected, strong or absent; the Sandbox's deterministic option
date: 2026-11-03 · by: determinism · issue: 0068
`VmOptions::weak_refs` / `Vm::set_weak_refs` / `vm_set_weak_refs`:
- `Collected` (default): D1513.
- `Strong`: no collection is observable. A WeakRef's pair and a FinalizationRegistry's
  targets table are allocated as ValueArrays instead of EphemeronArrays. The layout is the
  same (LEN, then the values) and every accessor works unchanged, but the collector traces
  them like any array. `deref` always finds its target, no cell ever dies, so cleanup is
  never due, and neither `Vm::collect`, the GC policy, gc-stress nor the cap changes
  anything a script sees. A table keeps its kind when it grows. Unregister tokens and the
  realm's list of registries stay weak (unobservable). The mode is a `Roots::FLAGS` bit
  (`ROOT_WEAK_STRONG`), so images keep it, and it applies to the weak references created
  after it was set. Memory: a target lives as long as its WeakRef or registry. The spec
  allows an implementation that never collects.
- `Absent`: the globals `WeakRef` and `FinalizationRegistry` are deleted (pi's
  deterministic contract, as quickjs-wasi leaves them out). Not reversible.
`Vm::new` and `from_base` apply the option, and `from_bytes` keeps the image's state. The pi
adapter now calls `vm_set_weak_refs(absent)` before the host globals instead of deleting
the globals in JS (0059's version).

Sandbox: `randomSeed` (default entropy: until now every Sandbox had the VM's fixed
default seed), `weakRefs`, and `deterministic: {startedAt, seed}`. In deterministic mode:
the seed, time zone 0, strong weak refs unless `weakRefs` says otherwise, and the clock
at `startedAt`. It moves only when an async result is delivered, to `opts.now` at that
moment (max with the current clock), through `vm_resolve_host_call_at` (D1670). The
Sandbox no longer sets the clock per fuel slice in this mode. The clock is in the
snapshot meta. Sync host functions don't move the clock: their position is deterministic,
but their time would be another input. A test shows the same results at fuel slices
1e6/1000/97.

## D1530: difftest `no-stress`; a stressed timeout is a failure again
date: 2026-11-01 · by: devflow · issue: 0053
Amends D1347. Eight corpus cases took 3–120+ s per stress mode in the debug gate (deep
recursion to a stack overflow, 200k rope operations, big arrays and BigInt loops: every
suspension copies the heap image and every GC-stress safepoint copies all live cells), so
they timed out under `--suspend-every`/`--gc-stress`. D1347 let such timeouts pass as
"not compared", which burned ~10 s of CPU per pair in every gate, made the compared set
depend on machine load, and would also have hidden a stress-only hang. Now those cases
carry `// difftest: no-stress <reason>` (the measured times) and skip the stress
variants; the plain run still compares them with qjs. All other cases get three times
their time limit under stress (`runner::STRESS_TIMEOUT_FACTOR`), and a stressed run that
still times out is a hard failure whose message suggests `no-stress`. The dquickts
importer writes the directive for the cases in its `NO_STRESS` table.

## D1531: qjs test262 results are cached
date: 2026-11-01 · by: devflow · issue: 0053
qjs is the oracle and fixed: for one qjs binary, test262 commit and runner driver, a
test always gives the same result, but a qjs run of the active group took ~25 s and
~130 CPU-s of subprocesses. `djs-test262 run --engine qjs` now answers from a cache file
in `vendor/qjs-cache/` (next to the test262 checkout, so all worktrees share it), named by
a hash of the qjs binary's bytes, the checkout's commit, the driver source and the memory
limit; entries are keyed by test path, strictness and harness files. Timeouts and crashes
are not cached (they can depend on load). A repeated active-group run takes 3.7 s (1 s of
it the run) and ~11 CPU-s. `--no-cache` reruns everything and rewrites the entries. A
checkout with local modifications at the pinned commit would be served stale results;
fetch-test262.sh refuses such a checkout.
## D1690: Running out of host memory is not a JS out-of-memory error
date: 2026-11-03 · by: heap · issue: 0069
`HeapError::OutOfMemory` now means only "the arena reached its cap". That is a function
of the program and the cap, and it becomes the catchable `InternalError: out of memory`
(D1600). When the host allocator refuses memory below the cap (the arena's backing store
cannot grow, or the collector's to-space cannot be reserved), the result is the new
`HeapError::HostOutOfMemory`, which is never catchable and never skipped:
- in a run, the fault (or a failed collection at a safepoint, which used to be ignored)
  sets a flag and zeroes the fuel; the run stops at the next instruction boundary with
  the new `Status::HostOutOfMemory`; `Vm::run` aborts the call (`Vm::abort`), so the VM is
  idle, its heap valid and usable;
- `Vm::collect` returns it, and the compiler reports `CompileErrorKind::HostOutOfMemory`
  (`eval` turns that back into the host fault, not a JS error);
- djs-wasm: status 5 HOST_OUT_OF_MEMORY; the Sandbox: `SandboxError` kind `memory`,
  "out of host memory"; pi's adapter: a `crash`.
Hosts should pick a cap they can always back (wasm32: about 1 GiB at most, docs/host.md),
so that out-of-memory stays deterministic. Test hooks: `Heap::fail_next_gc_reserve` (the
to-space) and `arena::FAIL_NEXT_RESERVE` (growth).

## D1651: TypeScript legacy decorators behind a compile flag, as thunks
date: 2026-11-03 · by: lexer · issue: 0065
`experimentalDecorators` / `emitDecoratorMetadata` are two `CompileOptions` booleans
(TypeScript only; strip-types ignores them), `djs run --experimental-decorators` /
`--emit-decorator-metadata` (implies the first, as qjs's CLI), djs-wasm `vm_compile` goal
bits 4 and 8, Sandbox options of the same names, and difftest directives
`experimental-decorators` / `emit-decorator-metadata`. The compilation follows qjs
(docs/compiler.md 7.11): each element's decorators are a parked arrow function (thunk)
called after the class, so they are evaluated in tsc's order and scope; parameter
decorators are compiled into the method's thunk by stepping out of the method while its
parameter list is parsed (a parameter list has no blocks, so parking and resuming it is
exact). The runtime is three builtins (`%ApplyLegacyDecorators`, `%DecoratorParam`,
`%DecoratorMetadata`) and closure kind 24 (`__param`'s function). Known divergence: qjs
ignores a parenthesized class decorator (`@(expr) class C {}`) in legacy mode; djs
applies it, as tsc.

## D1730: Canonical hash and run journals
date: 2026-11-03 · by: determinism · issue: 0073
`Vm::canonical_hash` (`vm_canonical_hash`, `Sandbox.canonicalHash`): FNV-1a 64 over the
image a collection leaves (popped stack slots cleared, as `Vm::collect`, without
finalization scheduling), computed on a copy made with `Heap::from_bytes_trusted`, so it
never perturbs the VM. Equal graphs give equal hashes whatever the fuel slices,
snapshots or collection points (D1090, D1601). The copy and its to-space cost about three
times the arena, briefly.

Journals live in the Sandbox (js/src/sandbox.ts) rather than in the core, since the host
values and the host function table are there. `runAsync/run(code, params, { journal })`
records, `replay(journal)` replays; both need `deterministic` (otherwise the clock follows
fuel slices and cannot be replayed). Entries: run (engine build, code, params, hash before),
sync (fn, args, outcome), async (callId, fn, args, outcome, clock) in delivery order,
check (delivered, issued, hash) at every idle point of the recording, and end. Replay
delivers one result per idle point, so it passes through every "quiescent after k
results" state. With the inbox (D1670) that state does not depend on how results were
batched, so each recorded check (taken at whatever k the recording happened to idle at)
has a matching point in the replay. Host calls are counted without the console, as pi's
`issued` is, and console output is not journaled (it is output, not input). A mismatch is
a SandboxError of the new kind `diverged`, saying where. pi keeps its own protocol (seq,
now), which maps onto the async entries one to one.
## D1700: A failing allocation collects and retries its instruction once
date: 2026-11-03 · by: heap · issue: 0070
Allocation still never collects inside Rust code (D1090: raw values would go stale).
Instead the interpreter retries at instruction granularity. Before `step` runs one of a
few instructions that allocate before they change anything observable, the run loop
copies its operands (at most 4) aside: `put_elem`, `put_field`, `define_field`, `add`,
`object`, and `call_intrinsic` of `Set`, `CreateDataPropertyOrThrow`,
`DefineDataPropertyOrThrow`, `ArrayCreate`, `ObjectCreate`, `NewRecord`,
`CreateIterResultObject` and `CreateArrayFromPair`. If the instruction then fails with
`OutOfMemory` (the cap, not the host's, D1690), the loop:
1. puts `pc` and `sp` back and writes the operands back into their stack slots (the
   instruction may have overwritten them), so every live value is rooted;
2. collects at that safepoint;
3. runs the instruction again.

A second failure is the real out-of-memory error (D1600). The retry happens at a
deterministic point, so the out-of-memory point stays a function of the program and the
cap at any slicing and across restores (vm/tests.rs `a_failing_allocation_collects_and_
retries`, D1601's test unchanged). `scripts/determinism/js/oom.js` at a 2 MB cap now
prints all dots (it printed `.EEEE…`).

Native builtins (0063) are not retried: an allocation failure inside a native frame is
still an immediate out-of-memory error.

## D1830: Property entries are 16 bytes; accessors live in an AccessorPair cell
date: 2026-11-03 · by: heap · issue: 0083
A `PropEntry` is `KEY: u32, FLAGS: u32, VALUE: Value` (was 24 bytes with a `SETTER`
slot that data properties never used). An accessor's `VALUE` is an internal ref to an
`AccessorPair` cell (CellKind 25: `GETTER`, `SETTER`), allocated before the entry is
written (D1700) and reused when the accessor is redefined. Pairs are never shared.
`heap::walk::verify` checks that every live accessor entry points at a pair. The reader
skips accessors as before, so it needs only the regenerated layout. The first step of the
shapes design (docs/objects.md §7): 0052 S2's first item.

## D016: wasm32 is the primary target for performance and memory design
date: 2026-11-01 · by: coordinator (owner direction) · issue: -
djs ships as wasm32 for all practical purposes. Memory and performance decisions are
judged first by their effect in wasm32 (node/V8's wasm tier), and native builds are a
development convenience. In practice:
- measure the important numbers in the wasm build (scripts/build-wasm.sh, node), not only
  natively;
- design layouts for a 32-bit address space and linear memory: u32 offsets, 4-byte
  fields where 8 aren't needed, compact cells, little padding; memory grows in 64 KiB
  pages and `memory.grow` has a cost (arena growth policy);
- wasm already bounds-checks every load and store against linear memory, so our own
  arena bounds checks (D1260) are a second check: removing or hoisting them (e.g. one
  check per cell, 0052 S3) matters more in wasm than natively;
- 64-bit values (the NaN-boxed `Value`) are native i64 operations in wasm and fine; no
  SIMD unless explicitly decided (0077: `+simd128` is cheap but drops SIMD-less
  embedders);
- code size counts (ARCH G2), so prefer table-driven or out-of-line code on cold paths.

## D1790: The gate's runners decide timeouts by fuel, not by the wall clock
date: 2026-11-01 · by: devflow · issue: 0079
Under load (load average 30–245 while several agents work) wall-clock limits failed the
gate spuriously: two test262 decodeURI tests that need ~3 s hit the 20 s limit, and each
such failure cost a full gate and a retry. Fuel does not depend on load, so it now
decides: djs-test262's djs engine stops a test at 2e9 fuel (`DEFAULT_FUEL`; the heaviest
active tests need 2–4e8, and a debug build runs ~1e8 fuel/s), djs-difftest's djs engine
a case at 1e9 (every corpus case needs < 1e8), and the e2e test a run at 20M (0053).
The wall clock stays as a hang guard for time spent outside the interpreter: 120 s per
test/case for djs (stressed difftest runs 3x), unchanged for the qjs subprocess engines (20 s test262, 10 s difftest), where it is the only limit.
`--fuel N` overrides the budget in both runners.

## D1820: The long tail: reference evaluation order and parameter scopes
date: 2026-11-04 · by: lexer · issue: 0082
- Compound assignment, `++`/`--` and logical assignment on `o[k]` load with the new
  `get_elem2` (`o k → o k' v`) instead of `dup2; get_elem`. It is one instruction. It
  throws for a nullish `o` before converting `k` (qjs's "value has no property"). It
  converts an object `k` in place, so the put reuses the primitive key and the
  `toString` runs once. Plain `get_elem` / `get_elem_method` on a nullish base also
  throw before converting an object key. `delete` keeps qjs's order (key first).
- `{ [k]: v }` converts `k` (`to_propkey`, a no-op for primitives) before `v` is
  evaluated. Class keys and methods have no user code between the key and the
  definition, so they keep converting at definition time.
- Binary numeric operators (not `+` or the relational ones) throw for a Symbol from
  ToPrimitive(lhs) before the rhs is converted (ApplyStringOrNumericBinaryOperator).
  This is one check in the interpreter's `primitive_operands` slow path.
- A function body with a non-simple parameter list gets its own var scope
  (`FuncState::var_scope`, docs/compiler.md 5.5). Its same-named `var`s are copied
  from the parameters at the scope's `enter_scope`, by resolution (`Scope::copies`).
  The criterion is "non-simple" rather than the spec's "has parameter expressions".
  The difference is not observable: without parameter expressions, no closure can see
  the parameter scope. A `catch` with a
  pattern gets a separate block scope (`ScopeKind::CatchBody`), with the
  LexicallyDeclaredNames early error checked across the two scopes.
- `(a?.b)()` keeps `this`. When an optional chain is the whole content of
  parentheses followed by a call, it leaves the callee and `this` (`Expr::Callee`).
- Strict arguments objects have the `callee` %ThrowTypeError% accessor. The function
  is now an `@intrinsic` (`installed_intrinsic(THROW_TYPE_ERROR)`).
- Script function declarations follow CanDeclareGlobalFunction and
  CreateGlobalFunctionBinding, as qjs does.
- New builtins: `Math.f16round` (direct f64 → binary16 rounding) and
  `Math.sumPrecise`. The sum is computed exactly in a 72×32-bit fixed-point
  accumulator of 2^-1074 units and rounded once, so intermediate overflow is exact.
  Also `Error.isError`, which reads the [[ErrorData]] slot as ObjectClassTag "Error"
  and is false for proxies.
- Smaller fixes: `Object.prototype` has an immutable prototype; `Object(v)` with a
  subclass new.target ignores `v`; Promise.prototype.finally's handlers call `then`
  with one argument; Number.prototype.toString has `length` 1; "no setter for
  property" drops the key, as qjs.
- The remaining failures are classified in TESTING.md "Known differences".

## D1791: Generated files are regenerated after a rebase, not merged; line lists union
date: 2026-11-01 · by: devflow · issue: 0079
In 6 h on main, docs/decisions.md changed in 73 commits, builtins/gen/prelude.js in 43,
resume-points.txt in 40, bytecode/intrinsics.rs in 32, intrinsics.d.ts in 29, the
ratchets in 23 and 19, and js/src/layout.generated.ts in 21; agents resolved the same
conflicts on every retry while main moved, and some squashed their branches to cope.
Now (see work/README.md "Merging and generated files"): append-style line files merge
with git's built-in `union` driver; generated files merge through
`scripts/merge-driver.py` (keep one side; for intrinsics.rs only when every conflict lies
in the `INTRINSICS` table) and merge-to-main regenerates them after the rebase
(`scripts/regen.sh`: bgen, `scripts/gen-intrinsics.py`, the layout test) and commits the
result before the gate. gen-intrinsics.py repeats the `table_matches_sources` rules in
Python, because when the table is stale djs-prelude's build script fails before that
test can print the replacement; the test stays the check.
The ratchets are not regenerated with `--update` after a rebase: that would accept any
new failure the merged code has, i.e. hide exactly the regressions the gate is for. A
union keeps both sides' lines; the cost is that a line one side removed in a conflicting
hunk comes back as a stale entry (reported, not failing) and could hide a regression of
that one test until the next `--update`.
