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
