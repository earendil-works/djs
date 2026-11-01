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

## D1792: The gate runs in one persistent gate worktree
date: 2026-11-01 · by: devflow · issue: 0079
merge-to-main.sh used to run the gate in the merging agent's worktree, whose target/
was built for its own branch (often hours and 100+ commits behind main) and, for the
gate's other profiles (wasm32 debug, wasm32 release clippy, clippy), was as old as that
agent's previous merge. Measured with a target 106 commits behind vs one at the previous
merge: clippy 7.0 vs 2.1 s, wasm32 release clippy 5.8 vs 0.2 s, ~40 CPU-s and 20–30 s of
wall time per gate under load, more after longer gaps. Now, under the lock, merge-to-main
checks out the rebased commit (detached, forced, `git clean` of untracked files) in
`.worktrees/gate` and runs the gate there, then fast-forwards main to that same sha. Its
target/ is always one merge behind. The agent's worktree only does the rebase and the
regeneration commit; its target/ is not rebuilt at gate width. The gate worktree is
created on the first merge (one cold build).

## D1793: Agents' runs are niced; the gate is not
date: 2026-11-01 · by: devflow · issue: 0079
The merge lock was held ~89 % of the time (10 merges/h), so the gate's cycle time is the
throughput of the whole project, while it competed for the CPU with every agent's own
test runs at the same priority. `scripts/timeout.sh` (which every engine run already
goes through) now runs its command under `nice -n 10`, except when `DJS_GATE=1` (gate.sh
exports it, so nested timeout.sh calls in the gate stay at normal priority) or the
command is merge-to-main.sh, gate.sh or with-merge-lock.sh (a niced process cannot
lower its niceness again). AGENTS.md asks agents to run cargo builds and test loops
through timeout.sh or `nice -n 10` as well.

## D1740: Fuel overshoot carries to the next run; a suspended intrinsic is charged once
date: 2026-11-03 · by: determinism · issue: 0074
An instruction is charged before it runs, so a slice ends with negative fuel when its last
instruction cost more than was left (a long string op, a large copy). The next run used
to start from the full slice, so small slices got further than large ones for the same
total. Now `Interp::FUEL_DEBT` (a Number, heap state) holds the overshoot when a run
stops mid-call (OutOfFuel, or parked on a host call). The next run that executes
instructions subtracts it first: with a debt larger than the slice it stops at once and
the rest carries on. When the VM is quiescent (Done, Threw, Idle) the debt is 0. The next
run starts a fresh slice anyway, and canonical hashes at those points (D1730 journal
checks) must not depend on the slicing. `Vm::call` and `Vm::abort` reset it.
`Vm::run_metered`'s `used` counts the fuel spent on instructions, without the debt paid.

A builtin that suspends mid-work (`suspend_in`: regexp exec/split/replace, D1332) is
dispatched again when it resumes and charged again. `suspend_in` now gives back that
dispatch's charge (the `call_intrinsic` op cost, or a native builtin's `CALL_FUEL`), so a
suspension costs nothing extra.

Result: a budget of k·B fuel, given in any slices that divide B, stops at the same
instruction in the same state (vm test, with and without image restores). Still not
slice-independent:
- fuel that depends on collection points (append buffers, D1610): `run` collects at
  its start (0071 removes that);
- regexp scans: the matcher charges one unit per character after the first, so a scan
  that stops for lack of fuel gets its first character free again when it resumes.
  Exact accounting needs a change in regexp/exec.rs (a follow-up).
## D1800: iter_next: iteration steps without result objects
date: 2026-11-03 · by: infra · issue: 0080
for-of and array destructuring step with a new op `iter_next L` (`it next -> r`). When
`next` (the value read once at GetIterator, as the spec calls it) is the installed
`%ArrayIteratorPrototype%.next` or `%StringIteratorPrototype%.next` and `it` is an
ArrayIterator object of the matching kind, the result `next` would return is a fresh
ordinary object with own data properties, so reading `done` and `value` back runs no user
code. The interpreter then produces the value itself and jumps to `L` (right after
`get_field value`): a present dense element of an Array, an in-bounds Number element of a
typed array, a code point of a string (kinds keys/values/entries). Holes, non-dense
elements, BigInt elements and other targets call the real `next` (its `Get` may reach a
getter or the prototype chain). At the end it finishes the iterator and pushes one
`{value: undefined, done: true}` without a call. Unlike D1351 this needs no @@iterator
check: only the identity of `next` matters, so it also covers `arr.entries()`, a
user-obtained iterator and `Array.prototype.values.call(ta)`. for-await keeps `call 0`.
`%GetIterator` of an Array/string whose @@iterator is the original (a data property, as
D1351) and `%IteratorClose` of an ArrayIterator with no `return` on its chain (lookup
without a Proxy, data `undefined`/`null` or absent) skip their frames
(`interp/run/iter.rs`). Fuel per step drops accordingly (D1633: fuel is not part of the
semantics). wasm: for-of over 200k numbers 1361 → 189 ms; +2.7 KB of wasm.
## D1840: Function.prototype is a builtin closure; ArraySetLength in [[Set]]; native DOMException, atob, btoa
date: 2026-11-05 · by: lexer · issue: 0084
- Function.prototype is created by the realm as a BuiltinClosure of
  `%FunctionPrototypeBehavior` (builtins/function.ts), a builtin that returns undefined.
  So it is callable (`typeof` "function", "[object Function]", `instanceof` with it as the
  right operand), not a constructor, and its properties are unchanged. It can be called
  once the prelude has installed the builtin.
- [[Set]] of an array's own writable `length` (interpreter `set_property`, and
  `ordinary_set` for Reflect.set and the builtins) converts a non-Number value as
  ArraySetLength does. A primitive converts inline (a RangeError for a non-length, a
  TypeError for a Symbol). An object goes to `%ArraySetLength(a, v, strict)`
  (builtins/array.ts), which converts it twice, ToUint32 and ToNumber, as the spec does
  (both call ToPrimitive), and then defines `length`. A `length` that became read-only
  meanwhile is a TypeError (strict) or false (Reflect.set).
- `DOMException`, `atob` and `btoa` are native globals with qjs's behavior and messages.
  The code is in builtins/web.ts, which sorts after error.ts so that Error is installed
  first; the Rust is in crates/djs-core/src/builtins/dom.rs.
  - A DOMException is an `ObjectClass::Error` object (so `Error.isError` is true) with
    two slots, name and message. Nothing else allocates an Error with slots, which is how
    the getters and `error_summary` recognize one. `name`, `message` and `code` are
    enumerable prototype getters.
  - The 25 legacy code constants are on the constructor and the prototype, enumerable and
    read-only. The constructor inherits from Function.prototype, not Error (WebIDL).
  - `atob` is FromBase64's loose mode on ASCII, `btoa` base64 of code units up to 0xFF.
    Both throw an InvalidCharacterError DOMException; a missing argument is a TypeError.
  - The pi adapter's JS polyfill of atob/btoa is gone (js/pi/vm.ts).


## D1710: GC points and raw images do not depend on the fuel slice
date: 2026-11-03 · by: num · issue: 0071
From 0055's G6 (P4). Before this change, a run with all its fuel at once and the same run
cut into slices left the same heap image only after a final collection. The raw arenas
differed, for two reasons:
- `run` collected at its start whenever a collection was pending. When a slice ended
  between the allocation that set `gc_pending` and the next safepoint, the collection
  came a few instructions earlier.
- An intrinsic that ran out of fuel mid-work (`suspend_in`: regexp exec, split and
  replace, D1332) allocated a progress record (the matcher state, plus split's or
  replace's progress). The record was allocated only when the slice ended inside the
  intrinsic, and it counted toward the GC threshold and the memory cap.

Now:
- **No safepoint at the start of `run`.** A pending collection waits for the next
  natural safepoint: a call, a return, a backward jump, or the handler of D1600.
  Allocations by the host between runs also just set `gc_pending`. The entry call
  (`Vm::call`) still does not collect.
- **Suspension records are unbudgeted and freed on resume.**
  - They are allocated between `Heap::begin_unbudgeted` and `end_unbudgeted`. The arena
    cap is lifted for those few allocations, and `gc_pending` keeps its previous value.
  - When the intrinsic resumes, it reads the record back and clears its state slot.
    `Heap::release_top` then truncates the arena to the record's first cell, provided the
    record is still the last thing allocated. The freed bytes are zeroed again, so the
    arena is byte for byte what it would have been without the stop.
  - If the host allocated in between, the record stays behind as garbage. Such an
    allocation already makes the image depend on the schedule.
  - Replace's partial output goes into a fresh string cell (`alloc_utf16`), never a
    shared one-character string, so freeing it frees nothing else.
  - After a restore, `gc_pending` is recomputed from the arena length, and the record
    counts in that. The resumed intrinsic releases the record before any safepoint and
    recomputes the flag.
- **Edge case.** A record allocated past the cap makes the arena briefly exceed it. If
  the host calls `Vm::collect` while the VM is suspended, the to-space (capped) can then
  fail. Both conditions are needed at once; it is accepted.

Result: raw images are identical for every e2e case and every scripts/determinism case.
Tested at slices max/997/101/13, plus 1 for the main cases, with and without a restore
at every stop. `crates/djs-core/tests/slices.rs` checks the 0055 cases (alloc_only,
regexp, regexp_props, iteration_runtime) at max/997/101/1 plus a restore schedule; it
also checks output and total fuel. An ignored test there sweeps all e2e cases. Removing
either part of the fix makes the test fail. Cost in the wasm build: +387 bytes
(+158 gzip).

## D1711: Regexp fuel can overdraw; backreferences are charged in full
date: 2026-11-03 · by: num · issue: 0071
The regexp matcher's fuel is now an `i64` (`exec`, `exec_at`, `resume`). A backreference
costs its length, and it used to be charged with `saturating_sub`. Whatever part did not
fit in the fuel left was forgiven, so the total fuel of a backreference-heavy match
depended on the slice size: `/(a{500})\1/` cost 4.98 M fuel at slice 997 vs 7.53 M in
one run. Now the backreference is charged in full, and the fuel may go negative. The
match stops at the next instruction, and `run_matcher` passes the overdraft on as
negative `Exec::fuel`, which D1740 carries as debt.

Scans (SCAN) and loops (STAR) were already additive. The character they get for free
when they stop is paid back on resume, because the instruction is dispatched again
there. `regexp::tests::chunked_fuel_is_additive` checks total fuel against chunk sizes
1 to 101 for scans, loops, lazy alternation and backreferences. `tests/slices.rs`
checks the same at the vm level, together with append-buffer, JSON and Array.from
workloads. The wasm build of `regexp-scan` runs about 0.7% more instructions (node,
3 runs each; within about 2x the noise).

## D1712: A suspended intrinsic's refund carries to the next run as a credit
date: 2026-11-03 · by: num · issue: 0071 (follow-up)
Background: `suspend_in` gives back the dispatch charge of a builtin that stops mid-work
(D1740), because the resume dispatches it again. When the matcher had used up the slice,
that refund left the run stopping out of fuel with fuel left over. `run` stored only
positive debt, so the remainder was lost, and every suspension cost the dispatch charge
once more. That moved budget stop points for regexp-heavy code: 0074's budget-form test
failed at slice 30030 for `/abc|bab?x|q/`, `/(?:ab){3}x/` and `/(?:a|b)*c/`.

Rule: `Interp::FUEL_DEBT` may now be negative, a credit. A run that stops out of fuel
stores `-fuel` whatever its sign. A run parked on a host call still stores only an
overshoot: fuel left over there was never owed.

Test: `vm::tests::budgets_with_collections_and_suspended_regexps` runs the D1740 budget
form (budgets of k·B fuel given as one run or as slices of 30030, 4620 with restores, 143
and 13; the state is compared after each budget) on those three regexps. It also covers
0074's append-buffer and JSON/Array.from loops, which pass since D1710.
## D1725: The determinism checks compare raw images too
date: 2026-11-03 · by: determinism · issue: 0072
Since 0071 (D1710, D1711) a run's raw heap image is the same under any slicing and
restores, not just its image after a collection. So e2e (plain vs suspension mode) and
difftest `--determinism` (the plain run vs the suspended variants at 97 and 1009 fuel)
now compare both hashes: the raw image at the end of the run and the image after a
final collection. A mismatch names which one differs. The raw comparison catches
garbage that depends on where a run stopped (a suspension record, a collection at
another instruction), which the collected comparison cannot see. GC stress stays
output-only (D1720). The native-vs-wasm check already compared raw images at each slice.

## D1801: Dense-array leaves for the element loops of Array builtins
date: 2026-11-03 · by: infra · issue: 0080
A present element in an Array's dense storage is an own default-attribute data property,
so HasProperty and Get of it run no user code. Leaves in `interp/run/dense.rs` run whole
element loops when every element they touch is such an element and the loop's other
steps cannot run user code either: `ArraySearchDense` (indexOf / lastIndexOf / includes,
strict equality or SameValueZero), `ArraySliceDense` (the copy into a species result
that accepts plain appends), `ArrayJoinDense` (primitives other than symbols),
`ArrayPushDense` (push(v) as a plain append, through `set_element_fast`),
`ArrayCanPopDense`/`ArrayPopDense` (heap `array_pop_dense`: only dense elements, writable
length, no index properties on the prototype chain), `EnumerableOwnDataProperties`
(Object.values/entries without enumerable accessors, no Proxy or other exotic
[[GetOwnProperty]] that runs code) and `TypedArraySetFromDense` (typed arrays from dense
arrays of primitives of the content type; the constructor uses an array of Numbers with
original iteration directly as the iterated list). Each declines with nothing changed
and the builtin's general loop runs; reads done before declining had no effect. Pure
checks come before the first mutation; where a store helper checks the conditions
(`set_element_fast`), its first call does and declines with nothing changed. The bgen
behavior stand-ins decline, so the general loops keep their qjs comparison. O(n) leaves charge one fuel unit per four elements
handled. A leaf that reaches the heap cap declines (its partial work is plain stores the
general loop repeats), so the allocation fails again at an instruction that collects and
retries (D1700) and the out-of-memory point stays where it was.

## D1802: Iterable consumers step without result objects
date: 2026-11-03 · by: infra · issue: 0080
`%IteratorFastReady(it, next)` (pure: the D1800 conditions, and the value is at hand) and
`%IteratorFastValue(it, next)` let the builtins that consume iterables (IterableToList,
Array.from's iterator path, the Map/Set/WeakMap/WeakSet constructors, Promise
combinators, Object.fromEntries, groupBy, array spread's general path, destructuring
rest) skip `next` and its result object per element; otherwise they call `next`. Map and
Set Iterators with their original `next` (now `@intrinsic`) also step in `iter_next`
(`collection::iterator_step_value`). The Map/Set constructors call `CollectionSet`
directly while the adder is the installed Map.prototype.set / Set.prototype.add
(`%IsOriginalCollectionAdder`) on the new collection, which is what that call does.

## D1803: bgen: try/finally and constructors in `@native`
date: 2026-11-03 · by: infra · issue: 0080
The Rust backend copies a finally block onto each way out of its try body: the normal
end, an exception (a handler region of its own that rethrows from its slot after the
copy), and each `return` (value parked in a slot), `break` and `continue` that leaves it
(loops record the finally depth at entry). Each copy is lowered in the try region outside
it, so an exception in a finally block goes outward. `try/catch/finally` is
`try { try/catch } finally`. Constructors needed nothing at runtime (construct_call
marks the native frame and stores new.target as for bytecode); the bgen guard is gone.
Set and Map constructors, GetPrototypeFromConstructor and AddEntriesFromIterable are
`@native` (and the latter two `@intrinsic`); no shipped `@native` uses try/finally yet,
so it is covered by the golden fixture (Fin) and the checker test.

## D1780: A block ASCII check in wasm32
date: 2026-11-03 · by: num · issue: 0078
Source: 0077's side finding. `<[u8]>::is_ascii` runs at 8–9 GB/s in wasm32 under node.
`heap::string::is_ascii` instead ORs 32-byte blocks as four u64 words and tests them once
per block: 37–46 GB/s from 1 KiB up, and about the same as std below 64 bytes. It is used
wherever the hot paths asked `is_ascii`: wire's ASCII strings, `Heap::alloc_str`,
`atom_str`'s hash and `str_eq_str`.

Natively the standard version is faster (SIMD: 12 ns against 18–24 ns per KiB), so
`is_ascii` picks the block version only on `target_arch = "wasm32"`. The block version is
tested on every target (`heap::tests::block_ascii_check`).

## D1781: Wire decode validates UTF-8 while transcoding
date: 2026-11-03 · by: num · issue: 0078
A non-ASCII wire string used to be read twice: once by `str::from_utf8` to validate it,
and once by `transcode` to turn it into Latin-1 or UTF-16. `transcode` now validates
strictly as it goes, applying `from_utf8`'s rules:
- no overlong forms;
- no surrogates;
- nothing above U+10FFFF;
- no truncated or stray continuation bytes.

It returns `Ok`, `InvalidUtf8` or `WrongLength`. The error messages stay the same: when
the units run out, the rest of the input decides whether the error is "invalid UTF-8" or
"string lengths do not match", as the two-pass order did. Keys (short) still go through
`from_utf8`. `wire::tests::transcode_matches_from_utf8` compares the verdict and the units
with `from_utf8` plus UTF-16 counting on 200k random byte strings, each at three
lengths. The string cell is now allocated before validation ends, so an invalid string
leaves a garbage cell, and a heap at its cap reports out of memory before invalid UTF-8.

Effect on decode:

| string | wasm, before → after | native, before → after |
| --- | ---: | ---: |
| 1 MB mixed (wasip1 bench) | 1.70–1.95 → 1.04–1.09 ms | 1.76–1.83 → 1.18–1.35 ms |
| 1 MB Latin-1 (wasip1 bench) | 2.8–3.2 → 1.9 ms | ≈ −10–20% |

`setGlobal` in djs-wasm: mixed 3.53 → 2.70 ms, Latin-1 3.65 → 2.62 ms, 4 MiB ASCII
1.74 → 1.35 ms (the last from D1780).

## D1782: `short_decimal` gives up early on full-precision numbers
date: 2026-11-03 · by: num · issue: 0078
D1262's fast path tried every k from 1 to 8 before it gave up on a number that has no
short form, such as `Math.random()` or most computed values. That cost 12–29 ns per
failure (0077).

Now one test comes first. When `tol8 = ulp(a)·1e8 < 1/4` and `a·1e8 < 2^53`, it computes
`a·1e8`. If that product is more than `2·tol8` away from its nearest integer, no k ≤ 8 can
round-trip, and Ryu takes over.

Why the test is sound: a short form `m/10^k` that round-trips lies within `ulp/2` of `a`,
so `m·10^(8-k)` is within `tol8/2` of the exact `a·1e8`. The product's rounding adds less
than `tol8`, so `a·1e8` stays within 1.5·`tol8` of that integer, which is then also its
nearest one. The test can only cost speed: Ryu prints the same digits.
`num::format::tests::short_decimal_early_exit_loses_nothing` compares the new path with
the old loop on 3M inputs (1.6M of them short).

Effect of `to_string`:

| input | wasm, before → after | native, before → after |
| --- | ---: | ---: |
| `Math.random()` | 157–174 → 120–124 ns | 60–71 → 57–66 ns |
| random bits | 180–194 → 146 ns | |
| short decimals | unchanged | unchanged |

In djs-wasm, `JSON.stringify` of 3 × 100k random floats went from 276 to 251 ms.

djs-wasm for D1780–D1782 together: +118 bytes (+34 gzip). A port of zmij's core (0077's
optional item) is not done; it remains a follow-up.
## D1890: pi adapter: snapshot() and the prelude image collect first
date: 2026-11-03 · by: snapshot · issue: 0089
`VmSession.snapshot()` and the prelude image (D1495) took `engine.image()` without a
collection: 936 KB for a session waiting on its first call and 933 KB per cached prelude
image, of which about 500 KB were live (0088). Now (js/pi/vm.ts, `Engine.collect` over
`vm_collect`):
- **Every cold start collects right after the prelude**, whether or not the image is
  cached. Cold and warm starts go on from the same heap, as before; the image is 496 KB.
- **`snapshot()` collects the session itself**, not a copy, then takes the image (498 KB).
  Continuing the session and resuming the snapshot then go on from the same heap, byte for
  byte. A collection that cannot reserve its to-space leaves the heap unchanged and the
  snapshot is just larger.

What a collection can change: later collection points. A deterministic script cannot
observe them (no WeakRef/FinalizationRegistry, D1680; fuel is charged for instructions and
their work, never for collections or layouts; a pi-vm test compares messages and interrupt
polls without a snapshot, snapshotted and continued, and resumed), except through the out-of-memory point of an allocation that is not retried
(D1700: native builtins and most leaf intrinsics fail at the cap without a collection). So
a run that was snapshotted can reach the memory cap at another point than its replay
without snapshots, and starts made before and after this change can too:
`DETERMINISTIC_VERSION` is 2 (the fingerprint's `deterministic 2`). In non-deterministic
sessions with weak references collected, a snapshot is an ordinary collection that may
clear WeakRef targets and queue FinalizationRegistry callbacks, which run with the next
delivered result.

Rejected: collecting a copy (restore the image into a scratch instance, collect, take its
image). The live session would stay untouched, but the resumed one would differ from it in
the same out-of-memory edge case, and every snapshot would pay a second restore.
Cost: `snapshot()` 0.05–0.1 → 0.4 ms; `resumeVm` of a waiting session 1.5 → 0.8 ms
(half the bytes to verify).
## D1860: Loop rotation and peepholes in the compiler; inc_loc/dec_loc; fast_more
date: 2026-11-05 · by: lexer · issue: 0086
- `while` and C-style `for` loops test at the bottom (one conditional jump per
  iteration). The parser keeps emitting in source order and then moves the phase-1
  byte ranges: phase-1 code refers to labels only, so just the fixups and the line
  entries move (docs/compiler.md 5.10).
- Peepholes run on phase-1 code before the re-emit (compiler/peephole.rs: the drops of
  expression statements, jump threading, `&&`/`||` in conditions, `not` before a jump)
  and in the re-emit where they need the storage (`inc_loc`/`dec_loc`, put+get as
  `set_loc`, `=== undefined` as `is_undefined`). They match consecutive instructions
  only, so no jump can land inside a match.
- New ops `inc_loc`/`dec_loc` (`i++;`, `++i` on an unchecked, uncaptured local). Not for
  `i += 1`, which concatenates on a string. An object local takes `%ToNumeric` into the
  local and re-executes (D1861); the conversion either completes or leaves the local
  alone.
- More simple ops (constants, `push_const`, `special_object` this, swap/nip/insert2,
  captures and Boxes, `not`, `strict_eq`/`strict_ne`, `is_undefined`/`is_nullish`, int
  `%` and bitops) run on `fast_more`, an out-of-line function the run loop tries after
  `fast_step`. As arms of `fast_step` itself they made the hottest ops slower in wasm
  (tight +6%, prop1 +5%: D1352's register allocation), after it they cost only the ops
  that take the general path (props +3%, a loop of slow ops).
- `get_field length` of an Array (its slot 0) or a string primitive is on the fast path
  (`fast_field`): the IC has no table entry to cache for it (arrays -11%).
- Fuel per iteration drops with the instruction count (D1633: fuel is not semantics).
- wasm (wmeasure, instructions vs 5459e97): tight -38%, loop -35%, call1 -22%, prop1
  -33%, calls -17%, forof -10%, arrays -9%, props -8%, strbuild -5%, pi-like -4%,
  jsonh/json +1%; executed bytecodes pi-like -9.5%, tight/loop -50%/-41%. wasm +9 KB.

## D1861: The Retry continuation
date: 2026-11-05 · by: lexer · issue: 0086
The slow paths that replace an operand and run the instruction again (`%ToPrimitive`,
`%ToNumber`/`%ToBigInt`/`%ToNumeric` converters) return through `Continuation::Retry(slot)`
instead of `Replace(slot)`. Both store the result in the slot; `Retry` resumes at the
instruction itself, so the caller's handler-table key and stack-trace line are `ret_pc`,
not `ret_pc - 1` (the end of the previous instruction). Before, a throwing conversion in
the first instruction of a `try` block escaped the block: `try { r++ } catch {}` with an
object `r` once `r++` is the single `inc_loc`.
## D1850: Leaf fast paths for everyday builtins (`@fast`)
date: 2026-11-01 · by: perf · issue: 0085
0081 F1 showed codemode time dominated by everyday builtins written in the builtin
language (`includes` = 7 bytecode frames, `split` 1,724 bytecodes per call). A builtin
tagged `@fast` in `builtins/*.ts` gets a Rust entry of the same name in
`crates/djs-core/src/builtins/fast.rs`: bgen emits `%MarkFast(f)`, which sets
`func_flags::FAST` and `FuncTemplate::FAST` (the table index, in the cell's padding, so
templates do not grow); a name without an entry fails the prelude, an entry without a tag
fails a unit test. With `ExecOptions::native_builtins` (the default, the native backend),
a call of such a function first runs the entry with `this` and up to 6 arguments; it
returns the result (the call completes, no frame) or declines and the builtin runs as
usual. `--backend bytecode` never takes them, so the gate's bytecode-backend run keeps
testing the TS path, and fast paths are part of the backend identity (D1633).
- Unobservable by construction (the D1351 rule): an entry accepts only arguments for
  which the algorithm cannot run user code or read anything user code could have
  changed (primitives for ToString/ToNumber/ToPropertyKey; objects only through leaf
  reads, e.g. own-property checks of non-proxies, or ArraySpeciesCreate's chain of data
  property and original-getter checks). Every case where the builtin would throw
  declines, so errors and stack traces come from the builtin. Out of memory propagates;
  a host refusal stops the run (D1690).
- Where: `Exec::call` (calls from natives, intrinsics, getters, `%Call`) and, for the
  `call` instruction, a check in the run loop after `fast_step` declines; not inside
  `fast_step`, whose register allocation is sensitive (D1352; measured: plain loops
  within ±1.5% in wasm).
- No collection inside an entry (a native caller keeps Rust locals across an inline
  call); allocations set `gc_pending` for the next safepoint. Fuel: 2 per fast call on top
  of the call instruction's.
- The set: the conversion helpers every bytecode builtin calls (ToString, ToNumber,
  ToIntegerOrInfinity, ToLength, ToPropertyKey, ThisToString, RequireThisCoercible,
  IsRegExp, ClampPosition, RelativeIndex, StrLen), String (constructor call, includes,
  indexOf, lastIndexOf, startsWith, endsWith, slice, substring, substr, charAt,
  charCodeAt, codePointAt, at, repeat, padStart/End, case, trim*, split and
  replace/replaceAll with string arguments and no `$` patterns), Number (call,
  toString, toFixed), parseInt/parseFloat/isNaN/isFinite, Math (max, min and the unary
  libm functions), hasOwnProperty, Object.hasOwn, Array.isArray, ArraySpeciesCreate.
  Array element builtins (push/pop/indexOf/slice/join) are infra's (0080).
- Strings built by the entries are written once (`string::join`, short ones gathered on
  the stack), not as ropes of intermediate strings.

Measured (wasm, node 25, instructions, 0085 Log): pi-like −50% (3.6x → 1.8x quickjs-wasi),
native −57%; per call `includes` 23.5k → 3.0k (qjs-wasi 3.2k), `Math.max` 25.8k → 1.8k,
`String(i)` 21.5k → 2.7k. Cost: +24 KB of wasm code (+2.9%).

## D1862: No dense opcode range
date: 2026-11-05 · by: lexer · issue: 0086
Rejected: renumbering the opcodes so that the `fast_step` ops (then the `fast_more` ops)
form one range from 0. It works as intended: wasm's `fast_step` dispatch becomes one
37-entry `br_table` right after the opcode load (instead of a compare tree plus four
tables), `fast_more` one table, and the wasm is 0.4 KB smaller. But node runs more
instructions (wmeasure, 3 runs, against 8fbd04f): call1 +7.5%, calls +6.8%, tight +3.2%,
loop +2.3%, higher-order +1.4%, arrays +1.3%, pi-like +0.5%, the rest unchanged. Moving
only the `fast_step` ops measured the same. TurboFan's code for the single table is not
cheaper than the compare tree, and the move perturbs the run loop (D1352). The `ops!`
order stays semantic; work/scratch/0086/renumber.py (not committed) redoes the reorder
for a later measurement, e.g. after the run-loop restructuring (0081 P6).
## D1783: Fast paths for array appends, strict equality and insert2/insert3
## D1783: `put_elem` appends on the fast path
date: 2026-11-03 · by: num · issue: gate time (coordinator)
test262's 613 RegExp property-escape tests were about 60% of the gate's test262 CPU. Each
one builds its strings with the harness's `buildString`, a JS loop of
`codePoints[length++] = codePoint` over about 1.1M code points, and it runs twice
(strict and sloppy).

`fast_elem` handled `put_elem` only on an existing element, so every append took
generic [[Set]]. `set_element_fast` already appends and fills holes where that is
unobservable: an extensible Array, writable length, no index properties on the
prototype chain. `put_elem` now calls it directly. (`strict_eq` and the `insert` ops,
the loop's other slow ops, came onto the fast path with 0086's `fast_more` meanwhile.)

`tests/corpus/language/elem-append-strict-eq.js` matches qjs, also under stress. It
covers appends, holes, frozen, non-extensible and length-readonly arrays, a setter on
Array.prototype, out-of-range typed-array writes, and `===` with NaN, -0 and mixed types.

## D1784: Bulk reads of dense arrays (fromCodePoint, apply)
date: 2026-11-03 · by: num · issue: gate time (coordinator)
`String.fromCodePoint.apply(null, list)` with 10,000 code points read every element
three times through `get_element_fast`, which checks the class and the bounds per
element:
1. `IsDenseArray`;
2. `AllCodePoints`;
3. `from_codes`.

`apply` (`CallWithArray`) read the list once more the same way.
`Heap::dense_array_values` now yields the first `length` slots of a dense Array straight
from one slice of its store. `AllNumbers`/`AllCodePoints` validate in one pass, testing
int32 values first, and `from_codes` and `CallWithArray` read the list in one pass.
Result: 270 → 135 native instructions per code point (release) for
`fromCodePoint.apply`.

## D1785: Dependencies at opt-level 1 in the dev profile
date: 2026-11-03 · by: num · issue: gate time (coordinator)
djs-core builds at opt-level 1 in dev (D010). At opt-level 0 and 1, rustc reuses generic
instantiations that an upstream crate already has. So djs-core's hottest paths called
libm's copies, which were built at opt-level 0, not inlinable, and full of debug
precondition checks: `Range::contains`, range iteration and `unchecked_add`.
`[profile.dev.package."*"] opt-level = 1` compiles libm, the only third-party dependency,
at opt-level 1 too. That alone cut a debug test262 property-escape run by 22% of its
instructions. Debug assertions and overflow checks stay on, as D010 wants.

Result of D1783–D1785 on the 613 property-escape tests, against main with 0086
(`djs-test262` built in dev, `-j 6`):

| | before | after |
| --- | ---: | ---: |
| instructions | 2.74 T | 1.33 T (−52%) |
| CPU | 97 s | 44 s |
| per test | 0.16 s | 0.07 s |

Other measurements:
- One test (Alphabetic): 5.70 → 2.95 G instructions in dev, 2.26 → 1.78 G in release.
- `fromCodePoint.apply` of 10k code points, 2,000 calls: 5.67 → 2.93 G in release,
  28.5 → 5.0 G in dev.
- wasm (node, `scripts/perf/wasm-run.ts`): Alphabetic 139 → 112 ms; the apply loop 463
  → 173 ms.
- djs-wasm size: +1.2 KB (+0.4 KB gzip).

The rest is the interpreter's per-op cost on the harness loop.
## D1900: Native builtins collect and retry at the heap cap
date: 2026-11-03 · by: infra · issue: 0090
D1700 retries an instruction after a collection when it reaches the heap cap; native
builtins (0063) now do the same for the same operations, at the native API level
(`Exec::n_retry` in interp/run/native.rs): on `OutOfMemory` the operation's arguments
are pushed onto the stack (rooted), the heap collects at that point, and the operation
runs once more with the arguments read back; a second failure is the real error. A GC
inside a state is safe only where no other heap Value lives in a Rust local of the
generated code, so the retrying forms are used in two places:
- a state's final call: `n_set`, `n_create_data_property_or_throw` and `n_call` of the
  intrinsics D1700 retries (Set, CreateDataPropertyOrThrow, DefineDataPropertyOrThrow,
  ...): their arguments are all the generated code holds at that point;
- a call that is a whole statement (`x = ArrayCreate(0)`, `return
  CreateIterResultObject(v, false)`, `r += s`): djs-bgen marks the outermost call of a
  statement (`Pure::take_root`) and emits `n_leaf_gc` for the leaf intrinsics in D1700's
  list and `n_concat_gc` for string concatenation; nested calls stay plain.
Nested retrying operations (a retrying `n_set` whose slow path is a retrying `n_call`)
retry only at the outermost level, so one failure costs at most one collection. As in
D1700 the retry point is a function of the program and the cap, so sliced and restored
runs agree (vm/tests.rs `native_builtins_collect_and_retry`,
scripts/determinism/js/oom_native.js).

## D1831: Shapes with strong transitions and a per-heap shape budget
date: 2026-11-03 · by: heap · issue: 0083
Ordinary objects keep their named properties in shape mode (a `PropValues` block, CellKind
27, plus a shared `Shape`, CellKind 26) or in dictionary mode (header flag
`OBJ_DICTIONARY` = 128, the `PropTable`), docs/objects.md 7. Transitions are **strong**:
a shape keeps its children (a `RefArray`, sorted by the key and flags of each child's
last descriptor, binary searched), so shapes never die. The approved design had weak
(ephemeron) transitions, but those make the tree depend on when collections ran:
- recreated shapes land elsewhere in the parent's list;
- a dead descriptor owner forces copies instead of sharing;
- limits count dead children.
The canonical hash (D1730) and replay would then see collection points. Growth is bounded
instead by `SHAPE_MAX` = 32 properties per shape and `SHAPE_BUDGET` = 4096 shapes per heap.
`Roots::SHAPES` counts them and is part of the image. Past either bound, an object whose
key set has no shape becomes a dictionary; existing shapes keep being shared.

Memory bound at the budget: a shape is a 24-byte cell plus 4–8 bytes of its parent's
transition array plus its share of the descriptors (8 bytes per property, shared along a
chain), about 40 bytes typically, so about 160 KB for 4096 shapes. The worst case is
every shape a second child copying a 32-property prefix (16 + 8 × 32 bytes), about
2.3 MB. There is no per-shape limit on children: with one, the root's 32 first keys would
be the only shapes ever made. Measured: the engine track and pi-like scripts create 0–16
shapes, and the difftest corpus at most 59 per case (median 3). Neither limit was hit.

Other rules:
- dictionary on attribute changes, array-index keys, more than 32 keys, the budget, and
  for the builtin namespaces, prototypes and constructors that `%Install*` targets, the
  global object, the lexical record, the constructor and symbol registries;
- deleting the last key goes back to the parent shape; deleting another key walks the
  tree again from that key's parent if at most one shape is new, else dictionary;
- own-property inline caches compare the receiver's shape (`IC_OFFSET_SLOT` slots with
  depth 0, emptied by the collector) and keep the verified index for dictionaries;
- `Heap::delete_own_property` and `remove_own_property_unchecked` return a `Result`,
  since a dictionary conversion allocates.

## D1870: Arena growth stops at the GC threshold; thresholds in 64 KiB pages; no to-space reuse
date: 2026-11-03 · by: num · issue: 0087
Source: 0081 F6. wasm linear memory never shrinks, so peak memory is what a run
costs. Before this change, the arena's Vec doubled past the GC threshold, and every
collection allocated a fresh to-space, shrank it and let it double again. The result was
strbuild at 57.8 MiB of wasm memory with a 15 MiB threshold.

- **Growth stops at the threshold.** `Arena` has a `target`, set to the GC threshold
  whenever the threshold changes. Below the target, growth doubles but never reserves
  past it. Past the target a collection is pending, so growth is by a quarter, not by
  doubling. After a collection, the to-space keeps its capacity unless that is more than
  twice the new threshold.
- **Thresholds are whole 64 KiB pages.** `next_threshold` rounds `live + growth` up to a
  64 KiB multiple when that stays within D1600's headroom rule. **This moves GC points**
  relative to before. They are still a pure function of the image state (D1601): the
  rounding uses only `live` and the limit, never allocator capacities, so slicing,
  restore and native-vs-wasm determinism are unchanged. All of these pass:
  - e2e image equality across modes;
  - `tests/slices.rs`, including the all-e2e sweep;
  - `native-vs-wasm.sh`;
  - difftest `--determinism`.
- **The to-space is not reused.** Keeping the from-space as the next to-space (0087's
  first plan) was built and measured, and it raised wasm memory: strings 15.3 → 19.2 MiB,
  arrays 30.6 → 37.1, strbuild 57.8 → 59.9. In dlmalloc a freed from-space also serves
  the Rust allocations made between collections (join buffers, JSON output); pinning it
  makes those take new memory. It saved no measurable instructions either.

wasm memory after a run (`scripts/perf/wasm-run.ts`, ONCE), before → after:

| workload | before | after |
| --- | ---: | ---: |
| strbuild | 57.8 MiB | 35.5 MiB |
| higher-order | 49.4 | 38.8 |
| arrays | 30.6 | 29.3 |
| json | 21.1 | 14.3 |
| jsonh | 21.8 | 13.9 |
| forof | 11.3 | 10.4 |
| pi-like | 7.2 | 6.4 |
| props | 5.2 | 4.8 |
| strings | 15.3 | 15.3 |
| loop | 2.8 | 2.8 |

pi bench (`js/bench/pi.bench.ts` with the instances' memory recorded), peak wasm memory:

| case | before | after |
| --- | ---: | ---: |
| script start, prelude image reused | 5.2 MiB | 3.9 MiB |
| tool-heavy, prelude image reused | 5.3 MiB | 4.4 MiB |
| tool-heavy, prelude run each time | 7.1 MiB | 4.9 MiB |

quickjs-wasi uses 1.6–1.7 MiB on the same cases.

Native peak RSS (`djs run --body`, steady over 3 runs) is mixed:

| workload | change |
| --- | ---: |
| higher-order | −11% |
| jsonh | −17% |
| strings | −13% |
| strbuild | +2–10% |
| forof | +13% |
| json | +2% |

wasm is the primary target (D016). wasm instructions are unchanged within ±1%, apart
from D1871's gains.

## D1871: No Rust allocations in short concatenation and fromCharCode
date: 2026-11-03 · by: num · issue: 0087
Source: 0081 F6. A short `'a' + i` spent more in malloc than in the concatenation.
`Heap::concat`'s path for results under `ROPE_MIN_LEN` (32 units) collected leaves into
up to three Vecs. It now walks the two operands with fixed arrays of 32 entries: every
pending subtree and every leaf holds at least one unit, so they cannot overflow. Before
the change, Vec churn was about 28% of strbuild's native time.
`String.fromCharCode`/`fromCodePoint` of up to 32 values (`from_codes`) now collects the
units in a 64-unit stack buffer and spills to a Vec only beyond that.

Effect, before → after:

| | before | after |
| --- | ---: | ---: |
| strbuild, native instructions | 3.00 G | 1.99 G (−34%) |
| strbuild, Rust allocations | 2.8 M | 100 k |
| strings, native instructions | | −4% |
| strbuild, wasm instructions | | −10% |
## D1901: JSON.stringify: plain subtrees and Dates in the fast path
date: 2026-11-03 · by: infra · issue: 0090
One `toJSON`, getter, replacer or class instance anywhere used to send the whole value
to the full algorithm in the builtin language (about 10x quickjs-wasi in wasm). Now:
- the full algorithm tries the fast path (`%JsonStringifyFastAt`,
  `json::stringify_fast_at`) on every object it reaches without a replacer function,
  after that object's own `toJSON` (not looked at again) and with its stack: reaching an
  object of the stack is the cycle TypeError, and the stack depth sets the indentation.
  A subtree that is not plain data declines (nothing observable happened) and the full
  algorithm goes on one level down, so only the path to a non-plain value is slow;
- the fast path serializes a Date itself while its `toJSON`, `toISOString`, `valueOf`
  and @@toPrimitive are the originals (data properties, no Proxy on the chain): then
  `toJSON` runs no user code and gives the ISO string or `null` (D1801's identity
  rule; the four are `@intrinsic` for that);
- an array replacer's PropertyList goes into the fast path as the key list ([[Get]] of
  each key: inherited and non-enumerable properties count, accessors decline);
- QuoteJSONString no longer sets up a serializer per string, and the key cache is
  filled on first use.
The full algorithm's functions are `@native` (10.5 KB of wasm, -20% on replacer
functions). wasm instructions (wmeasure.py, before → after, quickjs-wasi): one Date in
2000 records 18.1G → 1.2G (1.9G), indent + a getter 14.8G → 2.8G (1.8G), replacer
function + array replacer 8.9G → 4.1G (1.4G), plain 1.5G unchanged (2.2G).

## D1910: Dense-array loops, fromCharCode and the URI functions as `@fast`; entries decline at the heap cap
date: 2026-11-03 · by: infra · issue: 0091
- The dense-array loops of D1801 move to builtins/dense.rs (heap-level, shared by the
  leaf intrinsics of the builtin language and builtins/fast.rs) and become `@fast`
  entries: push of one item, pop, indexOf, lastIndexOf, includes, slice (through the
  ArraySpeciesCreate entry), join with a primitive separator, fill; plus
  String.fromCharCode of primitives and decodeURI/decodeURIComponent/encodeURI/
  encodeURIComponent of a primitive (a malformed URI declines, so the builtin throws).
  `this` must be an Array; any other case declines. Like D1850's entries they cost 2
  fuel per call, not per element.
- A `@fast` entry that reaches the heap cap declines instead of throwing (D1850 had it
  propagate): the builtin then runs and its own allocations collect and retry (D1700,
  D1900), so an entry never makes an out-of-memory error that the builtin would not
  have. The host's refusal still stops the run (D1690). The contract for entries: they
  allocate before they change anything, or make only stores the builtin repeats
  (fill's plain stores, slice into its fresh array). Test
  `fast_entries_decline_at_the_cap` (scripts/determinism/js/oom_fast.js).
- Numbers (main → this, Minstr; native measure.py / wasm wmeasure.py; qjs, qjs-wasi):
  pushpop native 4,935 → 2,720 (1,267), wasm 10,553 → 5,826 (4,744); indexof wasm
  1,043 → 731 (578); arrays wasm 18,172 → 13,382 (16,853); strings wasm 6,036 → 3,334
  (5,982). Per call natively: push+pop 6.1k → 3.9k, indexOf(len 8) 4.5k → 2.6k,
  fromCharCode(2) 8.6k → 2.8k, decodeURI 4.7k → 3.6k instructions. wasm +5 KB.

## D1920: Compiler throughput and size in wasm (0092)
date: 2026-11-05 · by: lexer · issue: 0092
Measured with scripts/perf/wasm-compile.ts / wcompile.py (compile-only in node, against
quickjs-wasi's COMPILE_ONLY) and the djs-bench `compile/` group. Changes, none of them
visible in the output (the disasm of typescript.js, highlight.js and the dquickts tests is
identical up to which of two equivalent `return` copies a jump takes):
- Bindings get a hash index by `(scope, name)` once a function has 32 of them
  (`FuncState::index`). Name lookup was a linear scan of the scope, quadratic in
  typescript.js's top-level closure (15% of its compile).
- The line table of a function maps positions with a cursor (`LineTable::line_col_from`)
  instead of counting each column from the line start, and an ASCII source counts bytes;
  `LineTable::new` scans eight bytes at a time.
- The stack check decodes each instruction once and returns the IC sites, so
  `alloc_template` no longer decodes the code again; `decode` reads a per-opcode layout
  table; the generated emitters all call one out-of-line `CodeBuf::encode(op, a, b)`,
  which builds the instruction in a local buffer and appends it once.
- The peepholes (D1860) run as one pass that backs up after a match, and thread jumps
  from the end until nothing changes; a second round only follows a rule that consumed a
  jump. Loop rotation finds the moved line entries by binary search.
- Keyword lookup is a perfect hash (a searched multiplier over the first two bytes, the
  last byte and the length) into a 256-entry table plus one compare, instead of a
  4.5 KB decision tree; the table's construction asserts there is no collision.
- Syntax errors and the TypeScript type error are built out of line.
Results (wasm, instructions per compile; base f166a25 → now; quickjs-wasi):
typescript.js 8,273M → 5,174M (4,561M), highlight.js core 55.4M → 48.2M (37.5M), a 7 KB
codemode script 23.2M → 20.2M (17.5M). Steady-state wasm MB/s: typescript.js 24 → 35
(qjs 40), highlight.js 42 → 54 (75), codemode 20 → 27 (35). Code: compiler 159.8 → 150.1
KB, compiler/ts 33.2 → 31.4 KB, lexer 17.8 → 14.2 KB; the shipped module 1,485,742 →
1,472,377 bytes, gzip 517,652 → 514,990.

## D1832: Addition caches and receiver-shape proofs; 5-word cache slots
date: 2026-11-03 · by: heap · issue: 0083
Part 3 of the shapes design (docs/objects.md 7.7). `IC_WORDS` is 5. Offset slots carry a
kind in bits 24..30 of word 0: 0 own (shape mode), 1..62 prototype depth, 63 addition.
- Additions: `put_field` and `define_field` (now with a `Cache` immediate, so
  `OPS_HASH` and the build hash change) cache old shape word → new shape. A [[Set]]
  addition also stores the receiver's prototype and the prototype epoch. It is filled
  only when the chain has no `key`, no proxy and no typed array, and the whole chain is
  marked `OBJ_IS_PROTO`. The slow path takes it only through `Exec::plain_addition`, a
  CreateDataProperty that no user code can interleave with.
- Prototype reads store the receiver's shape word; an equal word stands in for the
  per-hit `has_own_property`.
- The receiver shape word is 0 for dictionaries (never matches) and `EMPTY_SHAPE` (1)
  for shape-mode objects without a values block. Proxies are created as dictionaries.
- The hit checks are `ic::cached_get`/`cached_put` (plus `slot_at`), shared by every
  fast path, including 0093's tier-0 loop.

## D017: Note on duplicated decision id D1783
date: 2026-11-01 · by: coordinator · issue: 0095
docs/decisions-index.md (0095) found that D1783 appears twice in this log, under two
titles (from parallel branches). Both entries stand. References to "D1783" mean the entry
whose title matches the context; docs/decisions-index.md lists both.

## D1940: Where a pi VM's baseline wasm memory goes
date: 2026-11-03 · by: num · issue: 0094
I measured pi's `return 1` per export call with `djs_alloc_stats`. That is a new
`alloc-stats` feature of djs-wasm: a counting wrapper around the system allocator that
reports live, peak and largest allocation. The script was
`work/scratch/0094/account.ts`. Linear memory grows in 64 KiB pages and never shrinks,
and dlmalloc's realloc grows a block in place only if the top chunk already has room.
Otherwise it moves the block and leaves the old one behind as a hole.

| part (start reusing the prelude image) | before | after |
| --- | ---: | ---: |
| shadow stack (rustc's default) | 1024 KiB | 256 KiB (D1941) |
| data segment (base image 389 KiB of it) | 561 KiB | 561 KiB |
| staging buffer holding the image | 546 KiB | becomes the arena (D1942) |
| arena: the image reallocated to its GC target on the first allocation | +1093 KiB, old block a hole | one 1152 KiB block (D1942) |
| `verify`'s transient cell list | 137 KiB | 80 KiB (sized exactly) |
| memory after the run | 3.50 MiB | 2.13 MiB |

The pi bench (`js/bench/pi.bench.ts` with instance memory recorded; quickjs-wasi uses
1.56–1.69 MiB):

| case | before | after |
| --- | ---: | ---: |
| start reusing the prelude image (`return 1`, tool-heavy) | 3.50 MiB | 2.13 MiB |
| cold start that runs the prelude | 3.88 MiB | 3.81 MiB |

The cold start's peak is the collection after the prelude: from-space plus to-space.

**Rejected: a compressed base image.** A byte-level LZ in djs-prelude took the 398 KB
base to 159 KB and would have saved 239 KB of every instance's data segment. Decoding
cost 7.5 M wasm instructions (~0.5 ms) per `vm_new`, against 85 k for the copy. So
`Sandbox` creation and pi's cold start would have paid about half a millisecond each.
Left out; a faster codec is a possible follow-up.

## D1941: A 256 KiB wasm stack; the parser's deep paths fit it
date: 2026-11-03 · by: num · issue: 0094
rustc gives wasm32 a 1 MiB shadow stack. It is part of every instance's memory from
the start, so `crates/djs-wasm/build.rs` now links with `-zstack-size=262144`
(`DJS_WASM_STACK` overrides it for measurements). The only deep Rust recursion is the
parser, which counts nesting depth (`MAX_DEPTH` 1000, docs/compiler.md 2.4). Runtime
recursion (JS calls, proxies, bound functions, getters, JSON, the GC, `join`) uses the
heap or loops; ten such probes behave the same at 1 MiB and at 256 KiB.

I measured the wasm stack per nesting level by finding the deepest nesting that does not
trap at a 128 KiB stack:
- 130–160 B for statements;
- 230 B for patterns;
- 460–510 B for parentheses, arrays and objects;
- 1.4 KiB for classes.

At 1000 units, three constructs did not fit 256 KiB, and nested classes did not even fit
the old 1 MiB: a trap at the 498 levels the parser accepted. Fixes:
- **Smaller frames on the recursion paths:**
  - Decorator thunks and parked field initializers box their `FuncState`. `ClassBuild`
    went from 2.4 KB to 248 B, and `parse_class` from 3.8 KiB to 384 B of wasm stack.
  - The declaration arms of `parse_statement_list_item` moved out of line
    (`parse_declaration_item`). They had made its frame 368 B, 70% of a block level.
  - A C-style `for`'s init clause and `parse_for_in_of` are out of line.
- **Depth weights** for what is still big: a class body costs 7 extra units (about 110
  nested classes), a C-style `for` and a destructuring pattern level 1 extra each.

The deepest accepted nesting of every construct now needs at most ~170 KiB in wasm.
`js/test/nesting.test.ts` (gate, debug wasm) bisects each construct and requires the
limit's SyntaxError, never a trap. It fails at a 128 KiB stack.

## D1942: Restored arenas reserve their target up front; `djs_image_staging`
date: 2026-11-03 · by: num · issue: 0094
A restored arena's buffer was exactly the image, and its first allocation reallocated
it to the GC target, D1870's growth cap. In dlmalloc that is a move: a new block, plus
the old one left as a hole.

What changed:
- `Heap::over` now reserves up to the target right away; the reservation is best effort.
- `Vm::from_base` copies the base into a buffer that already has the arena's capacity.
- The new export `djs_image_staging(len, memoryLimit)` stages an image in a buffer with
  the capacity the restored arena will have (`Heap::restore_capacity`), so
  `vm_from_bytes` grows nothing. js/src/sandbox.ts and js/pi/engine.ts use it when the
  module has it.
- `verify` sizes its cell list exactly.

Results:
- Start reusing the prelude image: 2.75 → 2.13 MiB, on top of D1941.
- Base copy for `vm_new`: 1.69 MiB instead of 2.13.

Images, collection points and determinism are unchanged: capacity is never part of
the image.
## D1930: The tier-0 loop keeps the interpreter state in locals
date: 2026-11-01 · by: perf · issue: 0093
`Exec::run_hot` (interp/run/hot.rs) is the interpreter's main loop when
`ExecOptions::hot_loop` is on (the default). It loads `pc`, `sp`, `fuel` and the frame
data (code, its length, bp/hp/lp, consts, the stack's value base and capacity) into
locals, takes the arena as a local `&mut [u8]` (`Arena::hot_bytes`), and runs the fast
cases of the hottest instructions itself: locals, arguments, captures and boxes,
constants, the stack ops, int and float arithmetic and comparisons, `strict_eq`, `not`,
jumps, `iter_next` on Arrays, the leaf `call_intrinsic` cases, calls of plain closures
and returns into PushResult frames (D1931), and the inline-cache hits of fields and
globals (D1620/D1621, and 0083's shared `ic_get`/`ic_put`, additions included) and
dense elements. Everything else, and every declined case, goes to `step` (`step_once`,
with D1700's retry), called from inside `run_hot` with the state written back to
`Exec` first and read again after. It replaces D1350/D1352's `fast_step`/`fast_more`
(kept only in tests, as the reference).
- Why locals are safe: nothing `run_hot` itself does can collect or run user code.
  The leaf heap calls it makes (cache hits, an iterator step; a cached addition and
  `set_element_fast`'s append may allocate) end the slice borrow and re-take it after,
  and re-read `gc_pending`;
  no cell moves (no collection) and the stack cell never grows there. The borrow
  checker enforces that no `Exec` method runs while the slice is live.
- Exactness: the same fuel per instruction (checked at every boundary: it returns with
  fuel ≤ 0 and the run loop stops there), the same arena writes, and the same
  safepoints: frame entry, return and backward jumps complete, then, with a collection
  wanted, `safepoint()` runs at that point. Suspension points, collections and heap
  images are those of the old fast paths: vm/tests.rs `hot_loop_is_invisible` compares
  image hashes at every stop (fuel slices 1–97 with and without restoring the image, GC
  stress, collections pending at back-edges, frame entries and returns, the periodic
  exit forced at every backward jump) against `hot_loop` off (the D1350 paths), and
  fails when a fuel charge or a safepoint is changed.
- V8 tier-up: `run_hot` returns to the run loop after `hot_backedges` (2^14) backward
  jumps or general-path instructions, so a long slice still re-enters it and V8 (no OSR
  for wasm) runs TurboFan code: a single 10^12-fuel slice of engine/tight now runs
  8.6G instructions (main 16.8G, stuck in Liftoff). `--trace-wasm-compilation-times`
  shows `run_hot` (16 KB of wasm) and `step_once` (35 KB, `step` and `exec` inlined)
  compiled by TurboFan with pi's 10k and the Sandbox's 1M fuel slices.
Measured (0093 Log): wasm instructions −19 to −27% on plain bytecode (tight, loop,
call1, calls, prop1), −5 to −26% on arrays/strings/forof/pushpop/indexof, the rest within
±1%; native −19 to −29% on plain bytecode. The wasm code size is unchanged (−1.2 KB).

## D1931: Calls and returns in the tier-0 loop: the same frames, block access
date: 2026-11-01 · by: perf · issue: 0093
A `call` of a plain closure (no SLOPPY_THIS, NATIVE or FAST flag; the frame fits the
stack capacity) and `%Call` from `call_intrinsic`, and a `return`/`return_undef` into a
PushResult frame (bytecode or native caller), run in `run_hot`. The frame layout is
unchanged (docs/compiler.md 7.1: images, `Vm::backtrace`, native frames and the GC's
stack tracing do not change). The template's call data is read from one 28-byte window
(FLAGS .. CONSTS), the argument padding, the 6-word header and the locals are written as
checked blocks, and a return reads the header as one block and re-derives the caller
from stack[prev_bp] (a native caller is entered as `load_frame` enters it). Considered
and rejected: a seventh header word with the caller's template (a store per call, 8 B
per frame, an image format change); packing the header (its words are int32 Values so
that the GC traces the stack without frame knowledge). Native call+return
631 → 458 instructions (micro.py `call+return f(i)`).
## D1960: Phase-1 code as packed instructions; identifier atom cache (0096)
date: 2026-11-05 · by: lexer · issue: 0096
- Phase-1 code (`CodeBuf<Phase1>`, `Phase1Code`) is a `Vec<P1>` of instructions, 9 bytes
  each (opcode plus two raw immediates, `repr(C, packed)`), not bytes: the parser no
  longer encodes it and the resolution no longer decodes it. Phase-1 positions (`pos()`,
  the line entries, loop rotation, `Insn::pc` of phase-1 instructions) are instruction
  indices. `Phase1Code::bytes` encodes for listings and tests. Packed because the whole
  unit's phase-1 code lives until resolution: with 16-byte `Insn`s a fresh-instance
  compile of typescript.js was 17% slower (memory growth); packed it is 4% faster than
  before, at a 19% higher native peak (106 → 126 MB for 9 MB of source).
- The parser caches identifier atoms by their source text for the compile
  (`IdentAtoms`, open addressing over the bytes' hash): interning every identifier
  through the heap's atom table cost ~5%.
- The stack check's and the re-emit's per-function buffers are reused (`StackScratch`,
  `Scratch` in resolve.rs): ~1%.
- Measured and dropped: recording the final instructions during the re-emit so the
  stack check need not decode (+2%: the record costs more than the decode); a faster
  `encode` (neutral); the lexer has no re-lexing waste (about one `next_token` per
  token). Not done because each would gain less than ~3% or needs a format change:
  line/column computed lazily from the source at error time (~3.6%: a line-table format
  change plus a per-Script line cache), the remaining allocation (~10% spread over many
  small vectors), the peephole passes' bookkeeping (~1-2% each), the phase-1
  `finish` label check (1.7%).
Results, wasm instructions per compile (base 33ecb01 → now; quickjs-wasi), steady
state in one Sandbox: highlight.js 29.1M → 26.1M (21.3M), codemode 8.13M → 7.43M (6.24M),
dquickts tests 140M → 129M, typescript.js 4,479M → 4,058M (4,595M); a fresh Sandbox per
compile: highlight.js 50.4M → 47.5M, codemode 20.8M → 20.0M, typescript.js 5,234M →
5,044M. Disassembly of all inputs identical.
## D1970: Immutable ArrayBuffers
date: 2026-11-03 · by: infra · issue: 0097
The immutable ArrayBuffer proposal at the pinned test262 commit, as qjs has it:
`ArrayBuffer.prototype.immutable`, `transferToImmutable` and `sliceToImmutable` (no
species). An immutable buffer is a fixed-length ArrayBuffer whose slot 1 (the resizable
buffer's maximum) is `true` (heap/typed.rs `buffer_immutable`; no layout change). It
cannot be detached (`detach_buffer` refuses, `%BufferDetach` throws), resized or
transferred. Its typed arrays' elements are `{writable: false, enumerable: true,
configurable: false}`: [[DefineOwnProperty]] accepts only matching attributes and the
same value (SameValue, unconverted), [[Set]] of any numeric key fails before the value is
converted (`set_property`, `ordinary_set`, `set_own_data_value`, `set_element_fast`, the
put_elem conversion, Reflect.set, Object.defineProperty all check it), and `ta_set` never
writes one. Writing builtins check before reading their arguments
(`RequireMutableTypedArray`: fill, set, copyWithin, reverse, sort, setFromHex,
setFromBase64; DataView setters), and TypedArrayCreateFromConstructor rejects an
immutable result (from, of, map, filter, slice); subarray and the copying methods do not.
dquickts' test_immutable_array_buffer.js also needs SharedArrayBuffer and Atomics (a
non-goal) and sloppy `Function` code, so it stays in the corpus ratchet;
tests/corpus/builtins/immutable-arraybuffer.js is the same test without those lines.

## D1971: Error.prototype.stack is an accessor
date: 2026-11-03 · by: infra · issue: 0097
As qjs and the error-stack-accessor proposal at the pinned test262 commit have it: an
Error object records its trace in a slot (ObjectClass::Error has one slot now; a
DOMException three: the stack, then name and message, D1840) and has no own `stack`.
`Error.prototype.stack` is a getter (TypeError for a non-object, undefined for an object
without [[ErrorData]], including a proxy, without traps) and a setter
(SetterThatIgnoresPrototypeProperties: TypeError for a non-object, a non-string value or
Error.prototype itself; else an own data property, created or [[Set]]). This replaces
D1282's own non-enumerable data property. Effects: `Object.getOwnPropertyNames(err)` no
longer lists `stack` (as qjs); `JSON.stringify` and the reader's `Opaque` are unchanged
(the property was non-enumerable); `Vm::error_stack` (host display, `djs run`) reads an own
`stack` a script assigned, else the slot, without running JS; pi's prelude reads `stack`
through the getter (js/test/pi-vm.test.ts's stack assertions pass), and the difftest
corpus is unchanged. Objects that are not Errors (`%CaptureStack` of something else) still
get an own `stack`.

## D1972: Promise.allKeyed and Promise.allSettledKeyed
date: 2026-11-03 · by: infra · issue: 0097
The await-dictionary proposal at the pinned test262 commit, as qjs has it.
`PerformPromiseKeyed` (builtins/promise.ts) walks the own keys of the argument (strings
and symbols, [[OwnPropertyKeys]] order), takes the enumerable ones ([[GetOwnProperty]]
then Get, so proxies see their traps in the spec's order), resolves each value with
C.resolve and subscribes the Promise.all / allSettled element functions (builtin
closures 3, 4 and 5), whose Record gains a sixth slot with the keys. When the last one
settles, the result is a null-prototype object with the same keys
(CreateKeyedPromiseCombinatorResultObject). A non-object argument and every abrupt step
reject the returned promise. With 0097, test262's later group is empty (D1970, D1971,
this, and Symbol.unscopables, whose tests passed already).

## D1990: A restored arena reserves its GC threshold plus a slack
date: 2026-11-03 · by: num · issue: 0099
The cold-start peak of a pi VM (3.81 MiB) was not the collection after the prelude. It
came from an ordinary collection inside the prelude run:
1. The arena crossed its threshold. It was sized exactly to the threshold (D1942), and
   allocation went on until the next safepoint.
2. That overshoot reallocated the whole arena. In dlmalloc a reallocation is a move,
   so the old block was left behind as a hole.
3. The collection then allocated its to-space on top of both.

A heap made from the base or restored from an image now reserves
`t + max(64 KiB, t/16)` for its threshold `t` (`capacity_target`), and
`restore_capacity` (`djs_image_staging`) uses the same. The overshoot then fits without
a move. Only capacity changes, so images, GC points and determinism stay the same.

Two variants were compared on the pi bench (peak wasm memory, MiB, two runs each, same
in both):
- **slack everywhere:** the growth target after every collection gets the slack too;
- **slack at restore:** only the base or restored arena does. This is the one taken.

| case | before (0094) | slack everywhere | slack at restore (taken) |
| --- | ---: | ---: | ---: |
| warm, `return 1` | 2.13 | 2.06 | 2.06 |
| warm, tool-heavy | 2.13 | 2.06 | 2.06 |
| cold, `return 1` | 3.81 | 2.81 | 2.81 |
| cold, tool-heavy | 3.81 | 4.06 | 4.00 |
| `Sandbox` creation | 1.69 | 1.75 | 1.75 |

The warm numbers include D1991. Neither variant avoids the cold tool-heavy regression,
and it comes from where dlmalloc happens to place the blocks:
- The in-run collection now runs before the arena has grown, so the to-space it leaves
  as the arena is smaller (the from-space's length).
- The script's later growth then moves that arena once more. The freed from-space is too
  small to take it, and dlmalloc never returns memory.

Taken: slack at restore. It is never worse than slack everywhere and keeps no extra
capacity after collections. `vm_new` (39–40 µs) and warm-start times are unchanged.

Measured and rejected (dlmalloc's realloc grows a block in place only if the top chunk
already has room):
- a to-space sized for the next threshold: 5.5 MiB;
- reserving the new target right after the from-space is freed: 4.0 MiB.

The rest of the cold peak is Cheney itself: from-space plus to-space, about twice the
live data, plus holes from the moves. Removing that needs an in-place (sliding)
collector, a design question for the GC owner (0099's Result).

## D1991: `verify` keeps cell starts in a bitmap
date: 2026-11-03 · by: num · issue: 0099
`Heap::verify` runs when an image is restored. It used to build a list of
`(offset, kind)` per cell (8 bytes per cell) and a list of objects for the prototype-cycle
check. Now it keeps a bitmap of cell starts, one bit per 8-byte word (len/64 bytes); the
kinds are read from the headers, which are already checked. The cells are walked again
by their size fields. The cycle check uses two more bitmaps, "on the current path" and
"known to reach null", plus the path itself. For pi's 549 KiB prelude image that is
about 26 KiB instead of 80, and `vm_from_bytes` no longer grows wasm memory. The
binary search per reference also becomes a bit test.
## D2010: Error.captureStackTrace
date: 2026-11-03 · by: infra · issue: 0102
As qjs (and V8) have it: `Error.captureStackTrace(object, constructorOpt)` defines an own
`stack` on `object` (writable, configurable, not enumerable; on an Error it shadows the
D1971 accessor) holding the current trace without the builtin's own frame. With a
callable `constructorOpt`, the frames up to and including its topmost call are left out,
and the trace is the empty string when it is not on the stack. A non-object is a
TypeError ("not an object"), a non-extensible object without an own `stack` too ("object
is not extensible"). The trace comes from `Exec::stack_trace_from(pc, skip_prelude,
below)` through the leaf `%StackTraceString`. `Error.stackTraceLimit` (qjs has it, set to
10) is not added: the limit stays the constant of D1282.

## D2011: REPL completion reads the heap; `.step` pauses between fuel slices
date: 2026-11-03 · by: infra · issue: 0102
`djs repl` completes names with Tab from the heap without running JS
(crates/djs-cli/src/repl/complete.rs): globals from the global lexical record
(`Vm::global_lexical`, a new read-only accessor) and the global object's chain; after a
dotted path, the string keys along the chain of the object the path names, following
data properties only (an accessor or a proxy ends it). `.complete <text>` prints the list
for line-mode tests. `.step [n]` runs inputs in slices of `n` fuel and stops at a nested
`step>` prompt after each one, using only `Vm::run_metered` and `Vm::backtrace`; in line
mode the prompt reads the following input lines. `.decorators tc39|legacy|metadata`
selects the compile options of D1651 for later inputs.
## D1980: Privileged templates without debug fields (optional trailing fields)
date: 2026-11-03 · by: heap · issue: 0098
`cells!` takes an optional `min N` after a cell's fields
(`CellLayout::with_min_size`). The fields from offset N on are then optional: a cell may
stop before them, the walker (`walk::for_each_slot`, so the GC and `verify`) skips the
fields a cell lacks, and readers check the cell size. `min_size` is part of the layout
hash.

`FuncTemplate` uses this. Its debug fields `LINES`, `SCRIPT`, `SRC_START`, `SRC_END`
and `NAMES` moved to the end, from offset 56. `NATIVE` and `FAST` are u16 now (their
tables are far smaller). A privileged template without lines or names is allocated at
56 bytes instead of 80. Prelude functions show up in traces as hidden frames or
`(native)`, never with a file or line; their `toString` is `[native code]`; their
locals are never named. So nothing reads those fields for them.
`bytecode::template::debug_info` reads the five fields, null/0 for a short template.
Stack traces recognize prelude frames by the `PRIVILEGED` flag; they used to compare
the template's script with `Intrinsic::PreludeScript`, which stays for the non-privileged
case.

Base image 398,216 → 377,768 bytes (−20.4 KB, gzip −6.9 KB), the same in the wasm data
section; code +0.2 KB.

## D1981: The atom index stays at load ≤ 1/2 (measured)
date: 2026-11-03 · by: heap · issue: 0098
0052 S2 proposed a ~75% load for the atom index. A power-of-two index can't get there for
the prelude's ~1,880 string atoms (2,048 buckets would be 92% full). So 0098 tried
`hash * len >> 32` bucket selection with any length, a load limit of 3/4, and
`compact_for_image` sizing the index at 2/3 load. That saved 5.1 KB of the base image,
but atom lookups of non-atom strings (`o[k]` with computed keys) were about 10% slower and
new atoms 8% slower (release builds, interleaved). Not adopted: the index stays a
power of two at load ≤ 1/2. The patch is in work/scratch/0098/atom-load-34.diff.

## D2020: `@fast` builtins are not also `@native`, except Array.prototype.push
date: 2026-11-03 · by: num · issue: 0104
Fourteen builtins carried both a `@fast` leaf (D1850) and a generated `@native` state
machine (D1634). The leaf covers their common cases, so the native copy only ran when
the leaf declined:
- unusual receivers;
- conversions that run user code;
- holes.

13 of them now leave those rare cases to the bytecode version:
- array: indexOf, join, includes, slice, pop;
- string: at, charAt, charCodeAt, codePointAt, indexOf, startsWith, slice, substring.

Saved in djs-wasm: 26.2 KB raw, 4.2 KB gzip, 3.8 KB brotli (code 922,759 → 897,001).

Effect on the 22 perf workloads (wasm instructions, min of 2 runs): all within ±0.6%,
the noise.

Array.prototype.push keeps its native. `push(a, b)` and `push(...xs)` decline the leaf,
and without the native push-pop ran 2.0x the instructions and pi-like 1.1% more.

Rule for later: a builtin is `@native` only if the cases its `@fast` leaf declines are
hot in a benchmark.

## D2021: The number parsers have one instance; two-byte input is narrowed
date: 2026-11-03 · by: num · issue: 0104
`num::string_to_number`, `parse_float` and `parse_int` were instantiated for `u8` and
`u16` input. Callers with two-byte strings now pass them through `num::narrow_units`
first:
- JS whitespace and line terminators become a space;
- any other unit above 0x7F becomes 0x80, which no number syntax accepts and which is not
  whitespace.

Numbers are ASCII, so every result is the same; `num::tests::narrowed_units_parse_the_same`
checks this on 50k random strings, for all three functions and five radixes. The callers
are conv.rs `string_to_number`, number.rs `parseInt`/`parseFloat`,
heap/typed.rs element conversion and compiler/ts_decl.rs enum values.

Saved: 3.2 KB raw. Narrowing costs a byte copy, only for two-byte strings.

## D2030: Error.stackTraceLimit as in qjs
date: 2026-11-03 · by: infra · issue: 0105
`Error.stackTraceLimit` is a plain data property of Error (10, writable, configurable,
not enumerable; `@value` in builtins/error.ts). Like qjs (and V8), the VM reads it
without running code whenever it records a trace (`realm::stack_trace_limit`): the own
data property of the realm's original Error constructor (`%Constructors%.Error`, so
`MyError.stackTraceLimit` and a reassigned `Error` global do not count). A number is
truncated and clamped to `0..=2^31-1` (NaN and negatives are 0, Infinity is all frames);
any other value, an accessor (its getter is not called) or no property records no
frames, giving an empty `stack`. It applies to Error construction, the VM's own errors,
`Error.captureStackTrace` (frames left out by constructorOpt or as the builtin's own do
not count) and the `<input>` line of an eval SyntaxError, which counts as one frame.
`Vm::backtrace` (debuggers) keeps the fixed 10 (`STACK_TRACE_LIMIT`), whatever the
program set. `Error.prepareStackTrace` is not added (qjs has it; a follow-up).

## D2031: Call positions follow qjs: tail calls at the `return`, `new` after the callee
date: 2026-11-03 · by: infra · issue: 0105
qjs turns `call`/`call_method` that a plain return follows into a tail call, which
reports the position of the `return` statement; every other call reports its `(`.
djs has no tail calls but records the same position: `parse_return` re-marks the call's
line-table entry when the call is the last instruction of the operand and a plain return
follows in qjs's terms: a normal function (not async, generator or derived constructor),
no finally, for-of iterator or `using` scope to leave (`has_unwind`), and not inside a
try block or catch clause (`FuncState::open_trys`; qjs pops the catch offset first).
Eligible calls (`FuncState::tail_call`) are plain and member calls with an argument list
and tagged templates; not spread or optional calls, nor a call of the name `eval` (qjs
has an `eval` opcode). Concise arrow bodies keep the call's position (no `return`
statement, as in qjs). `new X(...)` and `new X` report the token after the constructor
expression (its `(`, or whatever follows), as qjs does, instead of the `new` keyword.
Codegen is unchanged; only the line table moves.

## D2032: `difftest: stacks` compares stack frames
date: 2026-11-03 · by: infra · issue: 0105
Difftest normally drops `    at ...` lines (D1100). A case marked `// difftest: stacks`
keeps them, normalized on both sides and in its `.expected`: a frame located in the case
(djs names it by the case path, qjs `<evalScript>`) reads `name (case:line:col)`,
`(native)` frames stay, and frames elsewhere (the prelude, qjs's `<cmdline>` driver) are
dropped. The qjs driver now loads the prelude with `std.loadScript`, so its frames carry
its path and differ from the case's `<evalScript>`; no other transcript changes (the
whole corpus still passes under qjs). The wrappers put different prefixes on line 1, and
line 1 of such a case is always its directive comment.

## D2100: The guide: Markdown in docs/guide, our own generator, generated parts checked in
date: 2026-11-03 · by: docs · issue: 0103
The human-facing guide lives in `docs/guide/` as CommonMark + GFM tables, ordered by an
mdBook-style SUMMARY.md, and is built by `crates/djs-docs` (owner's choice among own
generator, mdBook + preprocessor, GitHub-only Markdown; 0103 has the comparison). Parts
that derive from the code are generated regions (`<!-- gen:... -->`) and SVGs in
`docs/guide/gen/`, written by `djs-docs regen` from djs-core's tables (`LAYOUTS`, `OPS`,
`INTRINSICS`, `EXPORTED_TAGS`), the decision log and the compiler/VM themselves, and
checked in, so the Markdown reads the same on GitHub; scripts/regen.sh rewrites them
after a rebase. Example output (`gen:run`) is the exception: it is a doc-test, so the
gate's `djs-docs check` fails on a change instead of regenerating it (`regen --examples`
accepts it). `check` also renders every page and rejects broken links, unknown decision
IDs, opcodes and cell kinds. The site (`djs-docs build`, not in the gate) embeds live
examples that run on djs-wasm through `@djs/host` (`Sandbox.heapView`/`collect` were
added for its heap view). New dependency: pulldown-cmark (with memchr, unicase, bitflags,
pulldown-cmark-escape) in the djs-docs tool crate only; no tool needs installing.
Keeping the prose true is a process rule (AGENTS.md "Keeping the guide current").
## D2000: Typed locals of native builtins live in Rust variables, written through
date: 2026-11-03 · by: perf · issue: 0100
Generated natives (D1631) kept every local in a frame slot: each typed read was
`f64_of(ex.n_slot(i))`, two out-of-line calls in wasm (D1634). Now a local whose type
is bool, i32, u32 or f64 (`Storage::Reg` in emit_rust.rs) is also a Rust variable
`l<slot><b|i|u|f>`:
- typed reads use the variable; reads as a Value still read the slot, so the Values that
  natives pass on are unchanged bit for bit;
- every write goes to both (`let rv = ..; l3f = rv; ex.n_set_slot(3, num(rv));`), so the
  slots and the heap image are exactly what they were;
- a run of the function that starts at state S loads the registers live into S from
  their slots. bgen computes that per state with a liveness pass over the blocks (it
  scans the emitted code for the register names; writes are emitted only as `<reg> = `).

Why write-through and no spills:
- a run ends at every resume point that leaves the native, and at faults and backedge
  yields;
- with spills before those exits, the frame could already be another one (after
  `n_ret` or a pushed frame), and `?` exits are everywhere;
- write-through needs none of that, and the writes were there before anyway.

D1900 is unaffected: registers hold numbers and booleans, never heap references, so a
collection inside `n_retry` cannot leave one stale. Heap-typed locals stay in slots. The
test `native_registers_survive_collect_and_retry` checks this, with
scripts/determinism/js/oom_native_regs.js at caps where rounds fail without the retry.

Also in 0100:
- the state is a Rust local `st`, which `ex.pc` follows at the top of each round;
- a reentrant call that completes inline (`n_inline`) hands its result to the resume
  state through `Exec::n_res` rather than the operand stack. It is set only between
  `n_inline` and the next `n_result`, with nothing in between; debug builds assert this.
  A `Flow::Value` variant carrying the result instead cost 11 KB of wasm.

Effect, wasm instructions per element vs main:
- forEach −34%, sort −15%, reduce −18%, find −15%, map −13%;
- workloads: higher-order −10%, calls −12%, push-pop −6%, foreach −6%, pi-like −2%.

Cost: generated natives are 2.7 KB larger in wasm. The extra code is register copies
(phis) across the state loop; it buys back far fewer slot calls.
First merged as D2030 (3ff54985), which infra's Error.stackTraceLimit decision had
taken; renumbered, so older commits citing D2030 for the guide mean this one.

## D2050: Error.prepareStackTrace and CallSite objects, as qjs has them
date: 2026-11-03 · by: infra · issue: 0106
Like qjs (and V8), a stack recorded while `Error.prepareStackTrace` is set comes from
calling it with `(error, callsites)` and `this` = Error. Its result, whatever it is,
becomes the stack. If it throws, the stack is the default string. The function is an own
data property of the realm's original Error constructor, read without running code (an
accessor is ignored). It does not apply while a call of it is on the stack, so errors made
inside it get the default trace. The callsites follow Error.stackTraceLimit and
constructorOpt like the default trace (D2030, D2010).
CallSite objects are a new heap class, `ObjectClass::CallSite = 34`, with 7 slots:
function, name, file, line, column, native, constructor. Their prototype,
%CallSitePrototype% (a new realm intrinsic, no constructor), has qjs's methods
`isNative`, `isConstructor`, `getFileName`, `getFunction`, `getFunctionName`,
`getLineNumber` and `getColumnNumber` (null without a position), plus `@@toStringTag`
"CallSite". On anything else they throw "CallSite object expected".
The VM cannot call JS while it creates an error, so the stack is first a *pending*
internal Record of [callsites, default string, function, Error]
(interp/run/stack.rs). The prelude finishes it (`FinishErrorStack`, builtins/error.ts):
- the Error constructors and `Error.captureStackTrace` finish it at once;
- a VM error (or any error thrown with a pending stack, such as the builtins' `throw
  NewTypeError(...)`) is finished when it is thrown. `Exec::throw_fault` calls
  `%ThrowWithStack(e)` as if from the faulting instruction, with a `Retry` continuation
  at its pc, so unwinding uses that pc's handlers in bytecode and native frames alike.
  That function runs the prepare function and rethrows;
- any other error is finished on the first read of `stack`.
While the function runs, the error's stack reads undefined, as in qjs. Rust readers
(`Vm::error_stack`, the REPL, `error_stack_slot`) show a pending stack's default string.
The only timing difference from qjs: an error made by a builtin and never thrown (a
rejection reason) calls the function when its stack is first read, not when it is
created.

## D2051: SyntaxErrors from dynamic code start their stack at `<input>:line:col`
date: 2026-11-03 · by: infra · issue: 0106
Indirect eval, the Function constructors and JSON.parse report the position of a
SyntaxError as the first stack line, `    at <input>:line:col`, as qjs does. With
prepareStackTrace it is a CallSite whose function and name are null and whose file is
"<input>". The line counts against Error.stackTraceLimit. The compile leaves now return
the position (builtins/eval.rs `SyntaxAt`). The Function constructor's source has no
leading `(`, which qjs's has, so a column on line 1 is shifted by one to match. Messages
are unchanged; where they differ from qjs's, that is listed as a follow-up of 0106.

## D2052: A constructor's `prototype` is its last own property
date: 2026-11-03 · by: infra · issue: 0106
qjs defines `prototype` after a constructor's static properties, so
`Object.getOwnPropertyNames(Error)` ends with it. bgen now emits a constructor's
installs in this order: `%InstallConstructor`, then the constructor's statics (installs
whose target is the constructor), then `%InstallPrototype(target, key, parent)`, which
creates or looks up the prototype and defines `prototype` and `prototype.constructor`.
The install stays in one straight pass, with no reordering afterwards. Methods on
`X.prototype` come after, as before. Error's statics are in qjs's order (isError,
captureStackTrace, stackTraceLimit). Other constructors' statics keep source order, which
can differ from qjs's (follow-up). The base image changes accordingly: property order on
every constructor, a new realm intrinsic and the CallSite class (D2050).

## D2070: NamedEvaluation names only a function that is the whole initializer
date: 2026-11-03 · by: infra · issue: 0108
A name hint (`const f = …`, assignment, default values, property values, class fields,
destructuring defaults) used to name any anonymous function that started where the
initializer started, so `(function () {}).name`, `function () {}.bind(x)`, `(() => 0)()`
and `(function () {}) || x` named it after the binding. That was wrong in `.name` and in
stack frames. qjs, and the spec, name it only when the initializer *is* the function,
parenthesized or not. All hint sites now go through `Parser::parse_named`: it sets the
hint, parses, and then takes the name back from the function that took it (its
FuncState in `done`, or a class's constructor) unless `last_anon_fn` covers exactly the
initializer's span. The function is still parsed and compiled once; the template just
ends up without a name. TypeScript assertions are transparent, as in tsc's output (and
qjs): `x as T`, `x satisfies T` and `x!` extend `last_anon_fn` over the type, and
`<T>x` moves the hint onto its operand, like parentheses do.

## D2071: Syntax messages at the end of input and in dynamic functions follow qjs
date: 2026-11-03 · by: infra · issue: 0108
- qjs now shows the end of input as an empty token: `unexpected token in expression:
  ''`. djs said `'end of input'`.
- A statement list (block, function body, switch case, catch, static block) that hits
  the end of input reports the same message, because qjs tries to parse a statement
  there. It used to say `expecting '}'`. A class body reports `invalid property name`.
  TypeScript enum/namespace bodies keep `expecting '}'`, as qjs does.
- The Function constructors parse the parameters and the body apart, as the spec
  requires, so injected code is still rejected where qjs accepts it. The messages,
  though, are now what qjs's parse of the wrapped source gives:
  - parameters that close early: `expecting '{'`, or, when a `{` follows, the body is
    parsed and the message comes from what follows it;
  - a body that closes the function early: `expecting ')'`, or `unexpected token in
    expression: '}'` when the `)` is there and the wrapper's `}` follows.

## D2072: Constructor statics and namespace members in qjs's order
date: 2026-11-03 · by: infra · issue: 0108
`Object.getOwnPropertyNames(Array)`, `Number`, `Object`, `Promise`, `Symbol`, `Math`,
`JSON` and others now list their members in qjs's order. The builtins are spread over
files by topic, so source order cannot give it. Instead djs-bgen has a table of qjs's
order for the objects where it differs (crates/djs-bgen/src/qjs_order.rs). The installs
onto each listed object are sorted into it, in place. A key that is not listed keeps
source order after the listed ones, so a new builtin lands at the end until someone adds
it to the table. Prototype objects keep source order, with `constructor` first; qjs
installs it last. That is a follow-up.
## D2060: Natives call plain closures by writing the frame in place
date: 2026-11-03 · by: perf · issue: 0107
`%Call(f, this, ..)` from a native builtin (`n_call`, the callback call of forEach, map,
sort and the rest) used to take the general route:
- `n_call_once` pushed the arguments one by one;
- `call_intrinsic` dispatched;
- `Exec::call` re-read the callee and the template;
- the frame was then written.

For a plain closure (class Function, none of the FAST, NATIVE or SLOPPY_THIS flags),
`Exec::n_call_closure` now does all of it in one step:
- it reads the template once (`hot::template_head`);
- it makes the same two stack checks (`n_call_once`'s, then the frame top), so that the
  stack grows exactly as before;
- it writes the arguments as one block and the frame with `push_frame_head`;
- it charges the same `CALL_FUEL` and leaves the native, as before.

Heap images, fuel and stops are unchanged. The path is off when the hot loop is off, so
`hot_loop_is_invisible` (extended with callback natives: forEach with `this`, padded
arguments, map/filter/reduce/find/some/every, sort with an allocating comparator, a
callback that throws, nested natives) compares the two paths.

The return already goes straight back: `run_hot`'s return arm enters a native caller
with `code_len` 0, so the next round runs `native_step`. I measured a direct
`native_step` call in `run_hot`'s decline arm once more: it costs ~2% on tight loops in
wasm, as in 0100, so I left it out.

Also in 0107:
- `CellLayout::tail_capacity` is `#[inline]`. In wasm it was a call plus a runtime
  division in every `Heap::object_slot`; with a constant layout the division is a shift.
  This is the broadest win: tight −2.8%, call1 −3.4%.
- `hot.rs`'s `object_slot` checks the slot range without the division.
- The native fast paths test the class word in place (`Exec::is_class`) instead of the
  out-of-line `object_class`. Making `object_class` itself `#[inline]` cost 8.8 KB for
  0.5–3%, so it stays out of line.
- `AsNumber`/`AsFn`/`AsStr` have inline forms in natives (`n_as_number` …). Sort's
  comparator result went through `call_intrinsic` on every comparison.
- `type_error` and `bad_argument` build their faults out of line, and the well-known
  symbol names are a table. Together this is −1.3 KB.

Tried and dropped: reading a resume call's argument Values with one `n_srcs([..])` call
instead of one `n_slot` call per argument. It was 13% slower and 1 KB larger.

## D2150: Prototype objects in qjs's property order
date: 2026-11-03 · by: infra · issue: 0111
D2072's qjs-order table (crates/djs-bgen/src/qjs_order.rs) now lists the prototypes
whose order differed: Array, ArrayBuffer, the disposable stacks, DOMException, Date,
Error, Function, Iterator, Map, Number, Object, RegExp, Set, String, Symbol, Uint8Array,
WeakMap, WeakSet and %TypedArray%. Symbol keys are written `[Symbol.x]`. Three changes
let the table decide every key:
- `constructor` is an install like the others. `%InstallPrototype` sets only
  `f.prototype`. A new leaf `%InstallPrototypeConstructor(key)` sets
  `prototype.constructor`, emitted by bgen among the prototype's installs. By default it
  comes after the prototype's methods (qjs puts it last on most prototypes), and the
  table moves it where qjs has it elsewhere (String.prototype, %TypedArray%.prototype,
  DOMException.prototype). A prototype that installs its own `constructor`
  (Iterator.prototype's accessor) keeps it.
- Error.prototype's `name` and `message` are `@value` installs in builtins/error.ts
  rather than realm properties, so that they can follow `toString`. The native error
  prototypes keep theirs from the realm, already in qjs's order (name, message,
  constructor).
- The function-kind prototypes link `constructor` and `prototype` when the prelude
  installs their constructors (`wire_function_kind_constructor`), not at realm creation.
  That gives qjs's order on %GeneratorFunction.prototype% (constructor, prototype) and
  on %GeneratorPrototype% (next, return, throw, constructor).
What still differs are keys djs does not have: Annex B's HTML methods of
String.prototype, `setYear`, `RegExp.prototype.compile`, the `__defineGetter__` family,
and qjs's `Function.prototype.fileName`/`lineNumber`/`columnNumber`. The base image
changes with the new order.

## D2101: Status numbers are generated: counted from the repository or recorded with their commit
date: 2026-11-01 · by: docs · issue: 0113
README.md's status section and the guide's "Status in numbers" page are `gen:status`
regions of djs-docs (README.md is now maintained by `regen`/`check` too; it is not
rendered). Two kinds of numbers, so that `check` gives the same answer on every machine
and never measures:
- **Counted** on every regen from the repository: test262's active group and total (the
  runner's own grouping over vendor/test262; without a pinned checkout the region keeps
  its text and `check` warns), the djs and qjs ratchets, difftest cases and `skip-djs`
  cases, the base snapshot's size, `@builtin`/`@native`/`@fast` tags, opcodes, cell kinds
  and intrinsics.
- **Recorded** in `tests/scores/status.tsv` by `scripts/status.py`: the wasm module's
  size (`size`: bytes, gzip -9, brotli -q 11, code and data), the speed table against
  quickjs-wasi (`speed`: scripts/perf/wmeasure.py, instructions, n = 3), and numbers
  measured elsewhere (`set --at WHEN`, pi's test suite). Each row carries the commit or
  issue it was measured at, and the generated text shows it, so a stale measurement is
  visible instead of silent.
The README's speed workloads are now named after their files; push/pop is
`push-pop.js` (with string methods) and JSON with Dates `json-date.js`.
## D2090: Fused dense HasProperty + Get in natives; one-pass string search and in-arena joins
date: 2026-11-03 · by: perf · issue: 0110
**Fused HasProperty + Get.** The iteration builtins do
`if (HasProperty(o, k)) { .. Get(o, k) ..`. djs-bgen now puts a check state in front of
such an `if` (`Fsm::fuse_has_get`), but only when two conditions hold:
- the true branch starts with exactly the Get on the same operands (the code text of the
  two calls agrees);
- that branch has nothing before it.

The check runs `n_dense_pre(o, k)`:
- for an Array with an element at `k`, both calls give what their own fast paths would
  (true, the element);
- it leaves the element in `Exec::n_res` (D2000) and jumps straight to the Get's resume
  state, skipping both call states;
- otherwise it goes on to the HasProperty call as before.

Fuel, heap and resume states are unchanged: those fast paths charge nothing and write
nothing. Six natives use it: forEach, map, filter, every/some, both loops of reduce.
The golden fixture `NativeHasGet` has one fused and one unfused case.

**Strings.**
- `index_of` reads both strings in place. One-byte search uses a word-at-a-time scan for
  the first byte (`find_byte`; wasm has no SIMD, D016), then an inline compare. Before,
  it was a byte loop plus a `memcmp` call, and the needle was copied.
- `split` with a one-byte separator finds all pieces in one pass, instead of an
  `index_of` setup per piece.
- heap/string.rs (cleared by the coordinator): `str_view`, `str_view_flat` and
  `flatten` answer flat cells from one kind-byte read (`flat_fast`). Ropes and append
  ropes take the old code, and a view borrows the heap, so D1610's append buffers are
  untouched.
- `alloc_latin1_pieces`: one-byte joins and substrings copy within the arena once,
  rather than through a Rust buffer.
- Array `join` of strings builds its result from the pieces. Each element no longer
  becomes a Vec of units.

**Tried and dropped:** running small blocks in place at forward jumps (tail
duplication, "fewer states per element"). It cost +13 KB of wasm for 0–1% on most
iteration builtins (find −6%); state transitions in the wasm loop are cheap.

## D2200: The guide's site uses Elanor, vendored from its published packages
date: 2026-11-01 · by: design · issue: 0114
The site (djs-docs `build`) looks like Elanor, Earendil's design system. It uses
Elanor's published packages from pkg.earendil.com, pinned to one elanor commit in
`crates/djs-docs/elanor/package.json` and its lockfile. `scripts/elanor-update.sh
[SHA|main] [BRAND]` installs them and copies the files the site uses, unchanged, into
`crates/djs-docs/assets/elanor/` (VERSION records the commit and brand). djs-docs embeds
that copy, and the build copies it into the site.
- **Vendored, not installed at build time.** Builds on pkg.earendil.com expire 30 days
  after upload even when pinned, and installing needs an Earendil login, which agents,
  forks and the offline gate do not have. With the copy, `check`, `build` and the Pages
  workflow stay hermetic and secret-free, and an Elanor update is a reviewed diff
  (about 500 KB of files). Alternative, not built: docs.yml can get a one-hour install
  token for any earendil-works workflow through GitHub OIDC
  (`POST https://pkg.earendil.com/-/v1/login/github-actions`, `id-token: write`, no secret)
  and install `@main` at build time. That is always current but unpinned, and an
  upstream token rename would silently unstyle the site.
- **Used:** design-tokens (the `earendil` brand's tokens.css; `elanor` differs only in a
  warmer canvas, a one-word switch in the script), elanor-fonts (Mallos Serif, Commit
  Mono, SIL OFL 1.1), elanor-icons (a few Material Symbols, Apache-2.0), elanor-logos
  (the Earendil symbol in the header and the favicon, unmodified, per its brand
  guidelines). The licenses ship next to the files and the footer names them.
  **Not used:** elanor-react. Its CSS is CSS Modules with hashed class names and needs
  React. style.css ports the relevant rules (Prose spacing, CodeSnippet, Table,
  SideNavigation, TableOfContents, the docs app's layout) and refers only to tokens; a
  unit test fails if it uses a token that tokens.css does not define.
- **Highlighting:** our Rust highlighter marks the roles of `elanor-shiki-theme`
  (comment, punctuation, keyword, function, variable, string, number, type), and the CSS
  colors them with the theme's tokens. Running Shiki would add node and npm to every
  build for little visible difference.
- **Diagrams:** one stylesheet (svg.rs) for the generated diagrams and the hand-drawn ones
  in docs/guide/img (regen rewrites their `<style>` and `<defs>`), with every selector
  scoped to the diagram's root (`svg.dg-NAME`), because an inline `<style>` applies to
  the whole page. Colors are `var(--semantic-color-…, #fallback)`: inline on the site they
  follow the tokens and the light/dark toggle; as an `<img>` on GitHub they fall back to
  the token's resolved value from `assets/elanor/colors.tsv`, which the update script
  writes from the tokens catalog. After an Elanor update, `regen` refreshes the fallbacks
  and `check` flags any it missed.
## D2102: The guide is self-contained; checked by djs-docs
date: 2026-11-01 · by: docs · issue: 0115
Owner's rule: the guide (docs/guide, D2100) is read on its own site, so it never sends
the reader to something they don't have. `djs-docs check` enforces, on the pages listed
in SUMMARY.md (code blocks exempt, the decision log exempt as a record):
- no links to markdown files outside docs/guide (the log itself, `../decisions.md`, is
  a site page and stays allowed), and no mentions in prose or inline code of `*.md` files
  other than guide pages, of `work/…`, of `ARCH`/`TESTING`, or of issue numbers (`0088`);
- no bare goal numbers (`G1`–`G5`): the goals have a page, goals.md, linked by name;
- every decision ID resolves (as before) and links to its anchor on the decision log
  page, which now opens with an introduction for readers instead of the log's
  contributor preamble;
- repository files are linked only as source: one helper (`render::source_url`) makes
  every such link, to `blob|tree/<commit>/<path>` on GitHub at the commit the site was
  built from (`$GITHUB_SHA`, else HEAD), marked `class="source"`; under "Where to look"
  a code path that is not a link is an error.
Quoted developer comments in generated tables (opcode and intrinsic descriptions) drop
parenthesized references to documents and issues; generated status text names no
documents. Alternatives considered for decisions: per-page decision footnotes (would
duplicate the log) or no IDs at all (loses the trail to the measurements).

## D018: Adopt mark-compact for in-VM collections
date: 2026-11-01 · by: owner · issue: 0101
The owner approved heap's prototype (docs/mark-compact.md): mark-compact for every in-VM
collection, Cheney kept as `collect_canonical` for `canonical_hash` and the base image;
the gate's "after a final collection" comparisons use `canonical_hash`; naming as proposed
(`Heap::collect` = mark-compact). Measured: 10–50% lower peak memory on heavy workloads
for 0–5.5% more instructions and +6 KB of code.

## D019: Snapshots: compact images now
date: 2026-11-01 · by: owner · issue: 0088
Direction A from the snapshot research: collect, then compress against the recomputable
reference image (the base/prelude image as a zstd dictionary), with a reference
descriptor in the header and restore pinned to the engine build. B (journal replay as the
durable record) and the ARCH "Deltas" replacement are not decided yet.

## D020: The size budget is gzip relative to quickjs-wasi
date: 2026-11-01 · by: owner · issue: -
ARCH's "< 400 KB release wasm" target is replaced: the shipped module's gzip size must stay
within 1.5x quickjs-wasi's (today ~1.8x: 528 KB vs 287 KB), measured on every merge by the
size track. The base image is part of the module and part of the budget.

## D021: pi integration deferred; codemode corpus harvested with redaction
date: 2026-11-01 · by: owner · issue: -
Wiring djs into pi-private is deferred to later. The owner allows harvesting codemode scripts
from ~/.pi/agent/sessions into the difftest corpus, provided everything committed is
redacted (no secrets, credentials, personal data or private content in scripts or recorded
tool results).

## D2040: In-place mark-compact for in-VM collections; Cheney for canonical layouts
date: 2026-11-03 · by: heap · issue: 0101
The implementation of the owner's D018 (design and numbers in docs/mark-compact.md).
`Heap::collect`, and with it every in-VM collection, is a sliding mark-compact in four
passes (heap/compactor.rs). The passes run over side tables allocated before anything is
touched: a live-word bitmap (len/64 bytes) and a block table (u32 per 512 B, len/128),
plus a mark stack that grows on demand up to len/64 entries. A refusal from the host
returns `HostOutOfMemory` with the heap unchanged (D1690).
1. **Mark:** `gc_bits::MARK` in the header and the cell's words in the bitmap.
   - Leaf kinds (strings, Bytes, U32Array, BigInt) are not pushed.
   - On stack overflow a cell stays marked but unpushed; a rescan from the lowest such
     cell traces it.
   - References to flattened ropes are rewritten to the flat string while marking.
   - Ephemeron tables go to a fixpoint; dead pairs are then cleared in place.
2. **Block table:** a prefix sum of popcounts. A live cell at `a` moves to
   `table[a/512] + 8·popcount(bits below a in its word)`.
3. **Update:** every traced slot of every marked cell is rewritten. Nothing below the
   first dead word moves, so slots pointing there are left alone. Templates get
   `clear_offset_caches` before their IC ref is updated. Map/Set cells are remembered for
   the rehash.
4. **Slide:** runs of adjacent live cells move with one `copy_within`. The freed tail is
   zeroed (`Arena::truncate`), then the remembered Map/Set are rehashed (D1381).

Both collectors keep exactly the same cells at the same sizes, so live bytes, thresholds
and GC points don't depend on the collector. Surviving cells keep their address
(allocation) order: collecting along the way and collecting once at the end leave the same
bytes for the same mutations.

**Cheney** stays as `Heap::collect_canonical` (and `Vm::collect_canonical`). It is used by
`Vm::canonical_hash` on its copy (D1730), the base image (djs-prelude's build.rs and its
test), and `vm_new_compiled`.

**The gate:** the "after a final collection" comparisons of D1720/D1725 now compare
`canonical_hash`. That covers e2e plain vs suspension, difftest `--determinism`, and the
native-vs-wasm drivers, which print it next to the in-place collected hash. The raw-image
comparisons are unchanged.

**Measured against Cheney:**
- peak wasm memory −3% to −52% on 0081/0087's workloads;
- pi warm tool-heavy 3.13 → 2.13 MiB, cold `return 1` 3.56 → 2.94 MiB;
- wasm instructions ±0 to +5.5%;
- collection time 1.1–1.6×;
- code +6.0 KB (+2.0 KB gzip).
## D2280: The harvested codemode corpus
date: 2026-11-03 · by: infra · issue: 0118
Following D021, `tests/corpus/codemode-harvest/` holds 150 scripts that models wrote in
pi codemode sessions (`~/.pi/agent/sessions`: 2,620 codemode executions, 2,525
distinct). scripts/harvest/extract.py streams the session files and pairs each
`codemode` tool call with its result entry: the script, the nested calls (name, status,
arguments) and whether the run failed. The sessions record no nested results, so the
cases cannot replay real tool output. Each case answers its calls with synthetic results
of the recorded kinds, in the recorded order, and a recorded failure rejects.
build.py:
- keeps the scripts whose tools return plain strings (read, bash, ls, grep, find, 611
  after deduplication by token shape) or that call no tool;
- picks a set greedily by the language features it covers, rare features first;
- redacts the picks (D2281) and wraps each in a small header that defines the fake
  tools and runs the script as `async (tools) => {...}`.
The expected output comes from qjs on the redacted case.
Five cases differ in djs: `{...await tool()}` spreads a string result. qjs copies
nothing from a string primitive, against the spec; djs copies its characters, as V8
does. They are in the corpus ratchet as qjs bugs.

## D2281: Redaction of harvested scripts: arguments scrubbed, unknown words renamed
date: 2026-11-03 · by: infra · issue: 0118
Paths, commands, search terms and embedded programs live in the text of tool-call
arguments. Since the fake tools ignore their arguments, that text is replaced entirely,
letter by letter: same length and case, escapes kept. The same is done for any string
that looks like a shell command. Everything else is redacted by kind:
- comments are dropped;
- secrets, keys, tokens and connection strings are replaced (regexes per format);
- emails, URLs, hosts, IPs and home directories are replaced, and user names
  everywhere;
- path components are renamed (`dir1/file2.ts`, extensions kept);
- high-entropy strings become synthetic text of the same shape.
Then every word part of the remaining literals, regular expressions and identifiers
(camelCase and snake_case split) is checked: it is kept only if it is a lowercase word
of the system dictionary (with simple inflections), a JavaScript built-in name or a
programming or shell term, and is not on a denylist of private names. Every other part
is renamed to a pronounceable synthetic word, consistently within the case, so the
script behaves the same. An automated audit then looks for emails, URLs, IPs, hex
strings, key formats and capitalized name pairs in the written cases, and every case is
reviewed by hand. 9 picks were rejected in review: private tool names, component and
file names of private apps, a private file format, and internal function names. Their
source ids go into the exclude file, and the selection runs again.
## D2250: JSON without per-key allocations, escapes by runs; two-byte filter in one-byte search
date: 2026-11-03 · by: perf · issue: 0116
**JSON.parse**
- The key cache holds byte ranges of the source (it does not move during a parse)
  instead of a copied Vec per distinct key. It lives in a fixed array, so a parse
  allocates nothing for it.
- Escaped one-byte strings from one-byte input are written by runs:
  `write_escaped_latin1` copies up to the next backslash whole and decodes the escape in
  place. It is out of line, so that `write_string` stays small for unescaped strings.
- Tried and dropped: skipping plain bytes 8 at a time in the string scan, in every
  variant (in the unit loop, behind a counter, out of line, only after an 8-byte test
  at the start). It is 5–9% better on long strings, but every variant cost short
  strings (keys, record fields) 1–6%, and those dominate record-shaped JSON.

**JSON.stringify**
- The key cache keeps the quoted keys in one buffer (entries are ranges), and keys are
  quoted straight into the output. Before, there was a Vec per cached key and a
  temporary output per miss.
- The output starts at 256 bytes, and `quote` reserves its room once.
- "Writing in place" in the arena was not done. The final copy into the arena is a
  single bulk copy, and building the result at the arena frontier would need the fast
  path to never allocate in between: an invariant nobody enforces today.

**Strings**
- `find_latin1` tests the needle's first and last byte together, 8 candidate positions
  at a time, then checks candidates in full; the zero-byte test can flag false
  positives, never miss. With the first byte alone, a common letter (`t` of "throw")
  flagged most positions of a line.
- `flat_fast` (D2090) is `inline(always)`.
## D2103: Live examples: a textarea over the highlighted code, our own JS tokenizer
date: 2026-11-01 · by: docs · issue: 0119
Every live example of the guide is an editor (owner feedback: no separate view and edit
modes). It is a `<textarea>` with transparent text and a visible caret laid exactly over
the `<pre>` that shows the highlighted code; both share every property that moves a
glyph and wrap long lines (`pre-wrap`, `overflow-wrap: anywhere`), so the page never
scrolls sideways, and the pre sizes the editor. On every input,
crates/djs-docs/assets/highlight.js, a port of the JS/TS half of highlight.rs with the
same roles and classes, repaints the pre. A test runs it in node over every JS/TS block
of the guide and fails if its HTML differs from highlight.rs. Editing goes through the
browser's editing commands (`insertText`), so native undo, selection, input methods and
screen readers keep working: Tab/Shift-Tab indent and dedent (two spaces, also over a
selection of lines), Enter keeps the indentation (one step more after `{[(`), Backspace
in the indentation removes one step, Ctrl/Cmd-Enter runs, Escape hands the keyboard back
to the page (to the result tabs after the editor), Reset restores the original code (undoable). The code
re-runs 600 ms after the last edit; a run is capped at 20M fuel. Errors are shown with
their position: a SyntaxError's line and column, an uncaught Error's stack (the code runs
inside a `try` on its first line whose catch passes the stack out; only line 1's columns
move, and they are corrected), each with the source line and a caret, and the place is
underlined in the editor. Without JavaScript the page shows the highlighted code and the
build-time output, as before; only pages with live examples load live.js, and the sandbox
package and djs-wasm load on first use (focus, Run). Alternatives: CodeMirror 6 vendored
(about 150–200 KB minified for the editor and the JavaScript mode, a second highlighter
whose colors would have to be mapped onto the Elanor roles, and a bundle to pin and
update) for features a 5–20 line example does not need; highlighting in Rust in the
docs-only wasm (highlight.rs lives in the std tool crate, and the module loads only on
first use, so typing before it arrives would show plain text).
## D2260: Snapshots are compact containers: collected, zstd against a reference, pinned to the build
date: 2026-11-01 · by: snapshot · issue: 0117
Direction A of the snapshot research (D019). The Sandbox's `bytes()`/`fromBytes()` and the
pi adapter's `snapshot()`/`resumeVm()` write and read one container (js/src/snapshot.ts,
layout in docs/host.md "Snapshots"): a 40-byte little-endian header (magic `DJSN`, format
1, app, compression, the engine build hash, the reference kind, FNV-1a 64 and length of the
reference, the image length, the metadata and payload lengths), the host's metadata as
JSON (the Sandbox's host function names and clock, the adapter's session state), and the
collected image, compressed with zstd using a *reference image* as a raw-content
dictionary: the module's base snapshot (new exports `djs_base_ptr`/`djs_base_len`,
additive, ABI version unchanged) or pi's prelude image (D2262). Neither side stores the
reference: the build fixes the base, and the prelude's inputs fix the prelude image.

Restoring checks the container, the app, then the engine build before anything else, and
refuses another build with both hashes in the message ("snapshot from djs build X; this
module is build Y: an image restores only into the engine build that wrote it"); then the
reference (kind, hash, length), then zstd's content checksum ("corrupt snapshot"), then
the image's own verification as before. A host that must restore snapshots of older builds
has to keep those builds (an engine registry: not built). The formats before the container
(the Sandbox's `DJSS` and the adapter's `DJPI`, version 1: header, JSON, raw image) still
load. Measured on wasm: a pi session waiting on a tool is 2–9 KB instead of 546–566 KB
(its collected image; 936 KB before D1890), sessions holding tool data 63–404 KB (their
data, compressed), a new Sandbox 33 KB instead of 395 KB; a restore costs 0.1–0.3 ms more
for a waiting session, 1–1.6 ms more for one holding megabytes.

The small sizes depend on in-VM collections being mark-compact (D2040): sliding
compaction keeps cells in allocation order, so the cells a session inherited from the
reference stay at the reference's offsets (99.9% of a waiting session's 8-byte words), and
zstd encodes them as one long match; what is left is what the session changed. Under the
copying collector the same snapshot was 40 KB (one word in eight in place). Growing a cell
of the reference early in the heap undoes it: a new Sandbox's console grows the root
shape's transition list and the atom table, whose old cells sit near the start of the base
image, and the compaction then shifts everything after them (33 KB).

## D2261: Snapshot compression runs in the host: node's zstd at level 3; none without it
date: 2026-11-01 · by: snapshot · issue: 0117
`nodeZstd()` finds node's built-in zstd (`node:zlib`, node 22.15 and 23.8 on) through
`process.getBuiltinModule`, so the package still loads in a browser, and compresses at
level 3 with a window covering the reference and the image and a content checksum. Where
there is none (browsers, older node), snapshots are written uncompressed (the container
says so) and a compressed one cannot be read; `snapshotCodec` (SandboxOptions, VmHooks)
plugs in any zstd with raw-content dictionaries, `null` turns compression off, and
`bytes({ compression: "none" })` does it per snapshot.

Measured (prelude reference): a waiting pi session (546 KB image) is 126 KB at level 1,
1.8 KB at level 3 in 1.2 ms (0.09 ms to decompress), 1.6 KB at levels 6 and 9, 1.5 KB at
19 in 40 ms; one holding 1500 records (1.8 MB) is 324 KB at level 3 in 4–6 ms, 301 KB at 6
in 11–13 ms, 265 KB at 19 in 300–400 ms. A window covering both changed nothing; long
distance matching made it worse (2.5 KB). Level 3 keeps `snapshot()` around 2 ms for a
waiting session and under 8 ms for one holding megabytes. Compressing in wasm instead was
measured with zstd-rs 0.1 (pure Rust, no_std, interoperable with C zstd both ways): sizes
within 1% of node's at level 3 (3% at 19), faster on small deltas (0.95 vs 1.26 ms) and
1.6–1.8x slower on megabytes, decompression 0.5–1.9x node's time, and +141 KB of module
(57 KB gzip; 54 KB, 24 KB gzip for the decoder alone). The module is already over its size budget (D020), and a 0.1
crate in the shipped engine is a risk, so it is not built in: a browser host that wants
compact snapshots can load such a codec as a separate module.

## D2262: pi's prelude image is the reference of the adapter's snapshots and is recomputable
date: 2026-11-01 · by: snapshot · issue: 0117
A session's snapshots are compressed against the prelude image it started from (a cached
one, or the one its cold start cached): 1.8 KB instead of 66 KB against the base for a
session waiting on a tool. Sessions
started without prelude images (`reuseImages: false`) use the base. To make the prelude
image a function of its inputs (the prelude, tools, globals, store, heap limit,
deterministic mode, and the build), the adapter runs the host globals and the prelude with
the clock at 0 in UTC; nothing there reads the time, and every run sets its clock before
its first slice, so scripts see nothing new. Two compilations of the module, and two
processes, produce the same image byte for byte (a test compiles the module twice; two
processes were compared by hand). `resumeVm` finds the reference
among the process's prelude images by its hash, or recomputes it from `options.prelude`
and `options.data` (the session's start data), refusing data that make another image;
without either it says what it needs. The FNV-1a 64 of a prelude image is computed the
first time a snapshot or a resume needs it (about 3 ms for 540 KB).

## D2263: Sandbox.bytes() collects first
date: 2026-11-01 · by: snapshot · issue: 0117
As the adapter does (D1890): the sandbox itself collects before its image is taken, so
going on with it and restoring the bytes continue from the same heap, and garbage does not
spoil the compression. A collection is invisible to scripts except through WeakRefs in
"collected" mode and the out-of-memory point of allocations that are not retried; journals
compare canonical hashes, which do not depend on collections (D1730).

## D2282: Difftest ratchet reasons are an excerpt around the first difference
date: 2026-11-04 · by: infra · issue: 0121
A corpus ratchet line (`tests/corpus/expected-failures.txt`) used to carry the whole
first differing line of both transcripts, which made a single harvested case's entry
several kilobytes long. A reason is now the two lines from 20 characters before the
first differing character, at most 80 characters each, with `…` where they are cut.
For an uncaught error, the message is cut the same way. The full diff is still in the
runner's normal output (without `--no-diff`). The ratchet file went from 28.7 KB to
1.2 KB.

## D2283: Harvested cases with structured tool results
date: 2026-11-04 · by: infra · issue: 0121
0118 kept only codemode runs whose tools return plain strings. The sessions do not
record nested results, but they do record the TypeScript declarations the model saw:
`toolsAdded` descriptions and `describeTool` output. extract.py collects them per run,
and scripts/harvest/synth.py parses a tool's result type (unions, intersections, arrays,
tuples, object and index types, `Record`, generics such as `CallToolResult<T>`,
literals) and builds a synthetic value of that type, with dictionary-word keys kept as
the script reads them. A run is used only if every tool it calls has a declared,
parsable result type. The other fakes:
- bash returns `{ exit_code, output, truncated }` when declared so or read so;
- edit, write and transcribe_file return text like the other string tools;
- `models` gets fakes for getModelsOfType/getAvailableOfType/getModelOfType, classify
  (answers by question type) and generateImages;
- `image()` prints `[image]`;
- `load(key)` only for literal keys whose value the session stored
  (`codemode-store` entries), answered with a synthetic value of the same shape.
Tool discovery, `fetch`, `require` and modules stay unfaked; such runs are left out.
This added 150 cases (h151–h300): 89 with MCP-style structured results, 25 with
`models`, 22 with `load`, 20 with `image`, 37 with edit/write. With them the corpus
covers 46 of build.py's 51 language features (36 before) and 2.3 times as many feature
occurrences.

## D2284: Extra redaction for structured-result harvests
date: 2026-11-04 · by: infra · issue: 0121
The new runs carry more private text outside tool arguments: prompts and classifier
instructions, search queries, notes, embedded Python and HTML. On top of D2281:
- every string literal longer than 40 characters is replaced letter by letter, like an
  argument; shorter literals are redacted word by word as before;
- ids and slugs that mix letters and digits (12 characters or more) become synthetic
  text;
- words with non-ASCII letters are always renamed;
- MCP server prefixes are renamed like any word, and REDACTION.md lists tool calls
  under the names the case uses, never the session's;
- runs of tools whose name contains `admin` are left out.
32 picks were rejected in review (private service and project names in surviving
words, a private app's file layout, recognizable prose); their sources are excluded and
the selection reran until every pick passed review and ran under qjs without a
SyntaxError.

## D2104: Every repository path in the guide is a source link, made by the renderer
date: 2026-11-01 · by: docs · issue: 0124
Owner's rule: every file or directory the guide mentions links to it on GitHub at the
build commit (`render::source_url`, D2102), not only under "Where to look". The renderer
does it, so sources and generators stay plain Markdown: inline code that names a tracked
path (`git ls-files`, work/ excluded; a full path, or the end of exactly one tracked path
such as `heap/cells.rs` or `gc.rs`; `path:40` and `path:40-52` link the lines) becomes a
link to the blob (file) or tree (directory) view; in plain text only full paths from a
top-level directory count (prose has slashes of its own); in inline code with spaces
(commands) and globs, each path is linked inside the code, a glob to its directory.
Markdown files are left to the self-containment rule (never linked). `djs-docs check`
fails, on the guide's pages, on a mention that starts like a repository path but names
nothing tracked (renamed or removed), on a partial path that fits several files, on a
link whose path text names another file than its target, and, as a final net over the
rendered HTML (headings, generated parts), on any path mention outside a link or code
block. `gen:excerpt` regions start with a link to the quoted lines (`#L223-L239`), so
their line numbers are regenerated with the excerpt. The decision log links full paths
that still exist and is not checked. Alternative: fail on unlinked mentions and make
authors and generators write `[`path`](../../path)` everywhere: more noise in the
Markdown for the same result, and generated text quoted from code comments (intrinsic
docs) could not be fixed at its source.

## D2041: An old region that minor collections neither trace nor move
date: 2026-11-03 · by: heap · issue: 0120
Phase 1 of docs/old-generation.md, built into the in-place collector (heap/compactor.rs).
`Roots::OLD_END` (B) and `Roots::MINORS` are in the image. Cells in `[0, B)` are old.
- **Minor collection** (`Heap::collect_minor`, `mark_compact(B)`):
  - Old cells count as live; they are not marked and do not move.
  - One linear walk over the old region reads every traced slot. A slot with a target
    ≥ B marks it as a root and is put on a *remembered list* (u32 per slot, a fixed 16 KiB
    reserved with the other side tables). The update pass rewrites only those slots.
  - If the list overflows, the update pass walks the old region again.
  - During the walk, old templates get `clear_offset_caches`. Old Map/Set/WeakMap/WeakSet
    with object or symbol keys ≥ B are flagged (`gc_bits::REHASH`, the unused `OLD` bit)
    and rehashed.
  - Old ephemeron tables are counted and joined to the fixpoint and the dead-pair
    clearing by a hop walk (headers only). Their slots are updated by another hop walk.
  - The bitmap and block table cover `[B, len)`; compaction starts at B.
- **Full collection** (`Heap::collect`, every explicit `Vm::collect`): the whole-heap
  mark-compact as before. B becomes the live bytes below the old B: the old region keeps
  its live cells and drops its dead ones.
- **Seal** (`Heap::seal`, `Vm::seal`, wasm `vm_seal`): a full collection, then B = len.
  `collect_canonical` also sets B = len and MINORS = 0, so canonical images are sealed:
  the base image's old region is all of it, and the canonical hash does not depend on the
  B a VM had.
- **Schedule** (`Heap::collect_auto`, the interpreter's safepoints): minor while
  B > 0, full every `FULL_EVERY` = 16th collection. A minor collection that leaves more
  than half the limit in use makes the next one full. Without an old region (B = 0, e.g.
  `Vm::new` before a seal) every collection is full, as before.
- **Determinism:** B and MINORS are image state, so whether a collection is minor or
  full is a function of the image (D1601). Sliding keeps allocation order, so after a full
  collection the bytes do not depend on where minor collections ran (gc_tests
  `scheduled_collections_do_not_change_compacted_images`).
- **Retention:** dead old cells, and the young cells only they reach, survive until the
  next full collection. `Heap::verify` checks that B is a cell boundary.
- **Measured** (main → branch; native collection time, min of 5, under load):
  props 20.4 → 10.0 ms, pi-like 29.3 → 18.1, strings 4.2 → 2.5, strbuild 6.0 → 4.2,
  forof 1.4 → 1.1, jsonh 5.4 → 4.5, higher-order 10.0 → 8.4, arrays 4.9 → 4.5,
  json 2.8 → 2.6.
  - Base-only heap (vm/tests.rs `base_region_scan_cost`): full 0.177 ms, minor 0.080 ms
    (0.155 ms before the remembered list), the bare scan 0.043 ms.
  - Wasm instructions: pi-like −5.7%, props −5.3%, strings −1.4%, the others within
    ±0.4%, except json +1–2%. A control build that only collects in full shows half of
    that json delta too: it is not collector work.
  - pi bench tool-heavy (prelude image reused) 3.65 → 3.28 ms.
  - Peak wasm memory is unchanged on 17 of 22 workloads, +0.1–0.2 MiB on four (the
    retained cells). strbuild went 25.5 → 31.9 MiB and strings 14.1 → 12.9.
    - Those two are placement effects in the host allocator: strbuild's arena sizes are
      within 0.6% of main's.
    - A variant whose first collection is full gives strbuild 25.5 but strings 18.6.
  - Code: +5.3 KB wasm (+2.1 KB gzip).

## D2042: Full collections do not promote; only seal points make cells old
date: 2026-11-03 · by: heap · issue: 0120
Measured alternative, rejected: every full collection sets B = len, so all survivors
become old (a generational scheme with promotion at full collections, plus a full
collection whenever the young survivors outgrew B). It cut collection time further on
workloads with large young survivor sets. It also raised peak wasm memory:
- higher-order 28.4 → 45.3 MiB;
- jsonh 6.8 → 12.0 MiB;
- pi-like 4.2 → 6.3 MiB.
Promoted data that died stayed in the arena until the next full collection, and
thresholds were computed over it. So B moves up only at explicit seal points (`seal`,
canonical collections) and shrinks at full collections. Promotion after surviving two
minor collections (aging) would limit that; it is not built.

## D2043: pi's later cold starts size the arena for the sealed prelude heap
date: 2026-11-03 · by: heap · issue: 0120
`Vm::from_base_reserved(base, opts, live)` / `djs_prelude::new_vm_reserved` / wasm
`vm_new_reserved(limit, live)` create a VM from the base whose arena already has the
capacity a heap of `live` bytes gets: its threshold plus slack, `Heap::restore_capacity`.
js/pi/vm.ts remembers the heap size of its last sealed cold start (module state, a hint
only) and passes it to the next cold start's `Engine.create`. The arena then never grows
while the prelude runs.
- **Before:** the arena grew while the prelude ran, and in wasm the moved block stayed
  behind as a hole.
- **Measured:** pi cold start peak memory 3.00 → 2.25 MiB, for `return 1` and for
  tool-heavy.
- **Limits:** the first cold start in a process has no hint and is unchanged.
- **Rejected:** reserving after `vm_new` moves the arena, which is the very hole the hint
  avoids (2.9–4.1 MiB).

## D2044: No card table and write barrier (old-generation phase 2) for now
date: 2026-11-03 · by: heap · issue: 0120
Phase 2 would mark dirty cards on traced stores into the old region, so a minor
collection scans only the dirty cards. With D2041 that can save at most the old walk,
about 0.04 ms per minor collection for the 400 KB base:
- props: ≤ 6 ms of its ~10 ms of collection time, ~2.5% of the run;
- pi-like: ≤ 6.5 ms of ~18 ms, ~2%.
Against that, a compare on every traced store of the interpreter, the hot path of the
same workloads (props is mostly property stores), and a barrier that must be complete,
raw stack writes included. Not worth it at today's base size. Revisit when the old region
gets much bigger, e.g. sealed pi images of several MB, since the walk is linear in B.
Aging/promotion (D2042) would help the young-dominated workloads (arrays, higher-order,
json), where a minor collection still costs about a full one.
## D2251: The base image is ordered by cell kind
date: 2026-11-03 · by: perf · issue: 0122
The base image (djs-prelude's build, now `Vm::prepare_base_image`) is compacted and
collected canonically as before. Then `Heap::order_for_image` (heap/compact.rs) reorders
its cells:
- the roots cell stays first;
- the rest are grouped by `CellKind` in its order;
- strings sort by content, and every other kind keeps its breadth-first order.

Every traced reference is rewritten, and as after a collection, the templates' offset
caches are cleared and Map/Set indexes are rehashed. The order is a function of the
canonical image, so the image stays canonical and deterministic, and the native and wasm
builds agree (checked by the gate).

Why: gzip finds far more repeats among like cells (templates next to templates, bytecode
next to bytecode) than in a breadth-first walk's mix. The image's gzip went from 105.3 to
96.2 KB, and the module's from 529.9 to 520.6 KB.

Snapshot containers (scripts/snapshot-research) are unchanged; an empty sandbox's
`bytes()` went from 33 to 15 KB. Nothing in the in-VM collectors or `canonical_hash`
changed.

## D2252: The wasm module ships the base image coded; hosts decode it once per module
date: 2026-11-03 · by: perf · issue: 0122
**The codec.** djs-wasm embeds the base image coded with djs-lzrc
(`djs-prelude/packed`; native builds keep the raw image, `raw`). djs-lzrc is our own
code, with no dependency:
- LZ77 with an adaptive binary range coder in the LZMA manner: four repeat distances,
  literals coded against the byte at the last distance after a match, distances as a
  slot plus extra bits;
- the encoder (djs-prelude's build.rs only) parses by a price-based search over the whole
  input;
- the decoder checks every read, then the output against the header's length and hash
  (FNV-1a over 8-byte words). It never panics on bad input, which a fuzz test of flipped
  bits, truncations and garbage checks.

The coded image is 79.0 KB, against 96.2 KB for gzip of the ordered image (D2251). The
module's gzip went from 520.6 to 507.3 KB; the decoder costs 7 KB raw (2.9 KB gzip) of
code.

**Decoding once per module.** An instance decodes the base at its first VM (or first
`djs_base_ptr`), which takes about 5 ms in node. Every sandbox and every pi engine VM is
its own instance, so the host decodes once per compiled module:
- `provideBase`, right after instantiating, writes the base an earlier instance decoded
  into the new one, through the additive exports `djs_base_inject_ptr` and
  `djs_base_inject`;
- the instance checks the length and hash, and on a mismatch refuses the copy and decodes
  its own;
- `rememberBase`, after the first `vm_new`, keeps the base per `WebAssembly.Module`
  (js/src/snapshot.ts, used by the Sandbox and pi's engine).

The ABI version is unchanged: old hosts never inject and still work.

Measured in node (`Sandbox.create`, ms):

| instance | before | after |
| --- | ---: | ---: |
| first | 7.9 | 13.2 |
| later | 0.18–0.22 | 0.33–0.38 |

The extra time on later instances is the 385 KB copy, its zeroing and the hash check.
Memory per instance is +64 KB (the coded segment stays in the data section beside the
decoded copy).

## D2331: The shipped module is built with remapped source paths
date: 2026-11-07 · by: bloat · issue: 0126
Panic locations embed the source file path of the code they come from, and for the
standard library inlined into djs and for registry crates (libm, dlmalloc) that was an
absolute path of the build machine (`/Users/<name>/.rustup/...`, `.../.cargo/registry/...`):
a privacy leak in a shipped artifact. `scripts/wasm-rustflags.sh` prints
`--remap-path-prefix` flags (the home directory to `~`, CARGO_HOME to `/cargo`, the
toolchain's rust-src to `/rustc/<commit>` like the precompiled std, the checkout to
`/djs`) as CARGO_ENCODED_RUSTFLAGS, and every build of the shipped module uses it:
scripts/build-wasm.sh, the gate's size step, djs-bench's size and wasm groups and
scripts/size/variant.sh. Cargo's `trim-paths` profile option would do this but is not
stable in cargo 1.99. scripts/size/budget.py fails the gate when the module contains
`/Users/`, `/home/`, `.cargo/registry` or `.rustup/`. The paths do not change between
machines or worktrees any more, so the module is reproducible across them.

## D2332: djs-wasm is no_std on wasm32, with a panic handler that never formats
date: 2026-11-07 · by: bloat · issue: 0126
On wasm32, djs-wasm is `#![no_std]` (all its dependencies already were): its own
`#[panic_handler]` records a message for `djs_panic_ptr`/`djs_panic_len` and traps.
Release modules record a fixed message ("djs-wasm panicked (an engine bug; ...)"), debug
modules (`debug_assertions`, which the gate's node tests use) the formatted panic message;
native builds keep the std panic hook. Without std, its panic machinery (hooks, payloads,
`PanicHookInfo` formatting) leaves the module: −3.4 KB gzip. Instance globals are plain
statics behind a `Local<T>` wrapper on wasm32 (one thread per instance, `unsafe impl
Sync` with that reasoning), thread-locals natively. The global allocator is the
`dlmalloc` crate (0.2.14, `global` feature) as a wasm32-only dependency: the same
allocator, at the same version, that std uses on wasm32-unknown-unknown, now named
directly. Core panics (`panic_bounds_check`, `expect`, RefCell) still build their
messages' `fmt::Arguments` at the call site, so core::fmt stays reachable on stable; D2333
removes their locations.

## D2333: The shipped module drops panic locations (-Zlocation-detail=none, the one unstable flag)
date: 2026-11-07 · by: bloat · issue: 0126
Every panicking call site in djs (slice indexing, `expect`, RefCell borrows, asserts)
materializes a `core::panic::Location` (file, line, column) at the call site, and on
stable LLVM cannot drop them even though the panic handler ignores them (D2332): ~1,300
locations, 17 KB raw of data. `-Zlocation-detail=none` makes them all one empty location.
Measured on the shipped module: 506,114 -> 497,603 bytes gzip (−8,511), raw 1,176,854 ->
1,159,910. The flag only changes panic metadata, not semantics. It is unstable, so the
shipped module's build sets RUSTC_BOOTSTRAP=1 with it, and only that build:
scripts/wasm-build-env.sh (which replaces 0126's scripts/wasm-rustflags.sh and also
carries D2331's path remaps) prints the environment, scripts/with-wasm-env.sh applies it,
and every shipped-module build goes through it (scripts/build-wasm.sh, the gate's size
step, djs-bench's size and wasm groups, scripts/size/variant.sh). djs-core's normal
builds, tests, clippy and the native tools never see it. DJS_STABLE_ONLY=1 drops the flag
and RUSTC_BOOTSTRAP (tested: the module builds and passes, at the larger size). Exit plan:
if a future rustc changes or removes the flag, set DJS_STABLE_ONLY behavior as the
default (delete the two lines in wasm-build-env.sh) and raise the size ratchet
(tests/scores/size.tsv) by the recorded 8.5 KB under this decision.
## D2253: The base image drops the prelude's internal names
date: 2026-11-03 · by: perf · issue: 0127
Compiling the prelude interns the names of its internal functions
(`ArrayPrototypeMap`, `IsCompatiblePropertyDescriptor`) and of its locals and
parameters as atoms. Nothing at run time needs them:
- a builtin's visible `name` is the property `%Install*` defines (D1433);
- stack traces skip prelude frames (D1980);
- privileged templates keep no local names.

`Vm::prepare_base_image` now drops them (vm/image.rs) after compacting and before the
canonical collection:
- the NAME of every privileged template is set to the null atom;
- every string atom the heap does not refer to is forgotten (`Heap::forget_string_atoms`):
  its entry becomes `undefined` and the intern index is rebuilt for the rest.

An atom is kept if anything refers to it:
- a property table key, a shape descriptor key, a symbol;
- a template's NAME or NAMES;
- an atom immediate in bytecode;
- any int32 in a constant pool (global declarations keep their atom ids there; other
  int32s are kept too, which costs a few extra atoms at most).

The well-known atoms and all symbols are always kept. A forgotten name that user code
interns later gets a new id. Atoms are indexes, so nothing else moves.

The image went from 385 to 322 KB raw, its gzip from 96 to 70 KB, and the module's gzip
from 509.5 to 487.1 KB. Unit tests, e2e, difftest (both backends, stress, determinism),
test262, the js tests (the wasm-compiled realm equals the base) and pi's codemode tests
(215/226, as on main) are unchanged.

## D2254: `@native` only where benchmarks or real codemode pay
date: 2026-11-03 · by: perf · issue: 0127
Applying D2020's rule to the remaining natives, with two kinds of evidence:
- `instr.py` counts over the 22 perf workloads;
- uses in the 320 codemode corpus scripts (300 harvested: `new Set(`/`new Map(` in 2,
  `.find(` 5, `toSorted`/`reduceRight`/`findIndex`/`some`/`every`/explicit `.next()` in
  none; `await` in 306, `.map(` 149, `.forEach(` 35).

13 builtins go back to bytecode only: Array.prototype.findIndex, every and some (with
EveryOrSome), reduceRight and toSorted; GetIterator; %ArrayIteratorPrototype%.next; the
Map and Set constructors (with AddEntriesFromIterable); Map and Set.prototype.forEach.

forEach, map, filter, push, reduce, sort, find, the JSON serializer, Promise reaction
jobs and GetPrototypeFromConstructor stay native.

The module's gzip went down 3.3 KB. Natively the workloads are unchanged (indexof,
json-date, pushpop, tight within 0.1%), and the wasm deltas were noise on a loaded
machine. The measured costs for the dropped ops are 1.3–2.5x on those ops, which neither
the workloads nor the corpus exercise.

## D2045: The first collection after a canonical image is full
date: 2026-11-03 · by: heap · issue: 0125
`collect_canonical` leaves `Roots::MINORS = FULL_EVERY - 1`, so a VM made from the base
image (or any canonical image) runs its first scheduled collection in full; `seal`
still leaves 0.
- **Why:** a VM from the base kills about 17.7 KB of old cells right away (probe on the
  djs-prelude base). The atom table's entries array (15.5 KB) is replaced when the first
  compiled script adds an atom; the rest are a PropTable (1.1 KB) and a U32Array (1 KB).
  Under D2041 they stayed until the 16th collection and shifted every threshold by about
  a 64 KiB page. That moved where dlmalloc placed the growing arena.
- **Peak wasm memory, mean (max) of 24 perturbed runs (D2048):**

  | workload | before D2041 | with D2041 | with D2045 |
  | --- | ---: | ---: | ---: |
  | strbuild | 25.50 (26.6) | 28.07 (31.9) | 25.51 (25.6) |
  | json-indent | 10.48 (12.5) | 7.55 (8.8) | 10.30 (10.3) |
  | strings | 14.25 (18.6) | 14.85 (18.1) | 14.90 (18.6) |
  | json-replacer | 6.99 (7.4) | 6.87 (7.6) | 6.76 (7.4) |

  The other workloads, over 8 runs, are back to their values before D2041, e.g. json
  10.0 → 9.9, json-date 11.2 → 11.0, pi-like 4.3 → 4.2 MiB.
- **Trade-off:** json-indent loses the lucky placement D2041 gave it.
- **Cost:** one full collection (~0.1 ms more than a minor one at the base's size) per
  VM that collects. Wasm instructions are within noise against main.

## D2046: No aging promotion
date: 2026-11-03 · by: heap · issue: 0125
Measured and rejected. The design (the prototype is not kept):
- `Roots::AGED` ends the survivors of one collection, `[OLD_END, AGED)`.
- A minor collection promotes the survivors of that range (OLD_END becomes their slid
  end, AGED becomes `after`).
- Full collections keep the ages, so collecting a collected heap stays a fixpoint.
- A full collection follows once the old region has grown by 1/2 (or 1/4, 1/8) since the
  last full one.

Results:
- **Wasm instructions** against no aging: props −2.1%, json −1.1%, jsonh −1.2%, pi-like
  −1.0%, strbuild −0.8%, strings +0.7%, the rest ±0.3%.
- **Native collection time:** higher-order −12–15%, pi-like −12–21%, props −7–9%.
- **Peak memory** (8 perturbed runs) got worse:
  - pi-like 4.3 → 6.3 MiB;
  - strbuild mean 27.4 → 33.2 MiB (max 49.8);
  - json-replacer 6.8 → 12.3 MiB;
  - a 1/8 trigger keeps pi-like at 4.3 but not strbuild.

Cells that survive two collections still die soon: pi-like's per-task data, a string
buffer that is replaced as it grows. Adding a full collection whenever a minor one would
grow the arena (D2047) brings memory back, except pi-like at 5.8 MiB, but costs +2–6%
instructions. The gain is about 1% of instructions; the memory risk is several MiB.

## D2047: Arena growth policies that did not help
date: 2026-11-03 · by: heap · issue: 0125
Measured on peak wasm memory and rejected. The baseline is D2041 with D2045's cause still
in place.
- **Reserving the threshold's capacity right after a collection:** arrays 28.6 → 36.3 MiB
  (shrinking to the live size first) or 49.1 (not shrinking), foreach 9.8 → 14.5,
  json-indent 7.5 → 9.8. Memory is committed that the program may never use.
- **Shrinking the capacity to the live size after each collection:** arrays 45.4 MiB,
  higher-order 36.9.
- **Growth by 1.5× or 1.25× instead of doubling up to the threshold:** json-date
  improves (7.3 / 5.9 MiB), but with 1.25× json rises to 15.3 and jsonh to 10.6; strbuild
  stays at 31.9.
- **Slack in the base image's atom table:** strbuild mean 27.6, strings 17.3.
- **A full collection instead of growing after a minor one** (threshold rising): memory
  as before D2041 except strings, but +2–6% wasm instructions on growing heaps (arrays,
  json, jsonh, pi-like, strings).

The allocation traces show the cause. Every big peak is an arena realloc that dlmalloc
cannot do in place, so the old and new blocks coexist (strbuild: 9.3 MB + 11.6 MB at the
top of memory). Which reallocs move depends on the exact sizes, so any policy change
moves some workload's peak by MiB either way. The robust fix is in-place growth of the
arena, a page allocator in djs-wasm (follow-up issue).

## D2048: Peak memory is compared as a distribution over perturbed runs
date: 2026-11-03 · by: heap · issue: 0125
`scripts/perf/mem-perturb.sh` runs each workload 24 times, run k with a kept string of
k × 1000 bytes allocated first, and prints min, mean and max of the peak wasm memory.
- **Why:** peak memory is deterministic but chaotic (D2047). One run per build reports
  the allocator's placement, not the policy:
  - strings is 14.1 or 18.6 MiB depending on a 1 KB shift;
  - json-indent 7.5 or 10.3 depending on 17 KB.
- **Rule:** memory claims ("no worse than main") compare these distributions against
  main, using `--root` for another checkout's reader. A workload is worse if its mean or
  max is clearly above main's, not if one run is (TESTING.md "Peak memory claims").
## D2334: Unicode property tables coded by structure
date: 2026-11-07 · by: bloat · issue: 0126
props.rs (the tables of `\p{..}`) drops from 57,820 bytes of range lists to 20,477
(GC_RUNS 5,202 + DATA 15,275), module 471,983 -> 466,299 bytes gzip (−5.7 KB; data
−37.6 KB raw, decoders +1.9 KB raw code). scripts/gen-unicode.py codes:
- General_Category as one partition of the code space (each code point in exactly one
  leaf): runs `len << 5 | leaf`, long runs with a varint. A leaf or group is one pass
  over the runs with a mask of leaves.
- each binary property as a base (a union of General_Category leaves and at most one
  earlier binary property) XOR a stored difference, the generator picking the smallest:
  ID_Start is the letters and Nl plus 19 bytes, XID_Start is ID_Start plus 40 bytes,
  Uppercase is Lu plus 18 (25,091 -> ~5,500 bytes for all 50).
- the emoji sequences by their structure, as libunicode does: Basic_Emoji's `c FE0F`
  pairs as the set of c, the modifier sequences as the set of bases (each with all five
  skin tones), the flag sequences as regional-indicator pair indexes, the ZWJ sequences
  as 440 templates with skin tone and hair slots and a code for which combinations exist
  (12,054 -> 3,203 bytes). The generator asserts each structure, so a Unicode update that
  breaks one fails loudly.
Scripts stay range lists (a partition was larger). Every lookup decodes exactly what it
did before: a scratch comparison of all 976 property names and values against the old
tables, and test262's property-escapes tests (613/613). Decoding is cold (regexp
compilation of `\p{..}`); the lexer's ID tables and the case tables are unchanged.

## D2335: Only NFC keeps quick-check tables; the other forms' checks follow from the decompositions
date: 2026-11-07 · by: bloat · issue: 0126
String.prototype.normalize's quick check had six range tables. NFC's two stay (localeCompare
checks NFC on every call). The other four are derived (exactly, asserted by
scripts/gen-unicode.py against DerivedNormalizationProps.txt): NFD_QC=No is "has a
canonical decomposition" (Hangul syllables included), NFKD_QC=No "has any
decomposition", and NFKC_QC is No/Maybe exactly where NFC's is or where the full
compatibility decomposition differs from the canonical one. These forms only reach the
check through an explicit normalize("NFD"/"NFKC"/"NFKD") on text beyond Latin-1, so the
per-code-point decomposition lookup costs nothing that matters. −3.6 KB raw, −1.0 KB
gzip. A unit test checks every code point: the quick check never says Yes for one that a
form changes.
## D2255: The shipped module inlines less (`-inline-threshold=40`), with hot helpers marked
date: 2026-11-03 · by: perf · issue: 0127
scripts/wasm-build-env.sh adds `-Cllvm-args=-inline-threshold=40` to the shipped
module's flags (opt-level s's default threshold is higher). Native builds are unchanged.

Measured on the shipped module:

| inline threshold | gzip | cost |
| ---: | ---: | --- |
| 40 | −3.2 KB | pi-like, json, calls +0.4% |
| 15 | −4.1 KB | pi-like +2% |

At 40 alone, push/pop slowed by 2–4%: helpers on @fast push/pop and frame entry went out
of line. They are now `#[inline]`:
- `Heap::object_slot_count`, `array_length` and `dense_len` (heap/object.rs);
- `Exec::array_push_dense`;
- `Continuation::to_flags`;
- intrinsics' `decline_oom`.

With those, the 22 workloads are all at or below main, by 0.4–9%, on a loaded machine
(pi-like −1.7%, pushpop −4.4%; work/scratch/0127/wm-n7.txt). The module is 2.8 KB gzip
smaller than main (472.0 → 469.2 KB together with D2256).

## D2256: The URI functions and String.prototype.substr run their builtins (no @fast)
date: 2026-11-03 · by: perf · issue: 0127
They are rare in JS generally (the URI functions) or legacy (`substr`, Annex B), and no
workload or codemode script uses them. Their @fast entries go; the bytecode builtins
call the same Rust (`UriEncode`/`UriDecode`) or are a few lines.

Measured natively, per call:

| call | with @fast | bytecode | cost |
| --- | ---: | ---: | --- |
| `substr` | 1.6k | 5.3k | 3.3x |
| `encodeURIComponent` | | | +32% |
| `decodeURIComponent` | | | +38% |

`toLocaleLowerCase`/`toLocaleUpperCase` keep their entries: they share their code with
`toLowerCase`/`toUpperCase`, so dropping them saves nothing. The everyday string entries
(charAt, charCodeAt, fromCharCode, codePointAt, repeat, lastIndexOf, replaceAll,
trimStart, fill) stay.

## D2336: Debug details of internal failures only outside the shipped module
date: 2026-11-07 · by: bloat · issue: 0126
Internal compiler errors (`{e:?}` of CodeError, StackError, TemplateError, Storage), a
corrupt heap image's VerifyError and the heap's invariant panics (CellKind, Atom,
Handle) formatted `Debug` values, which kept those impls and core::fmt's struct/tuple
builders in the module. `crate::detail::detail(what, &value)` gives `"{what}:
{value:?}"` natively and in debug builds and only `what` in the shipped module (wasm32
without debug_assertions, whose panic handler records no messages anyway, D2332). The
wire error and djs-wasm's "cannot create a VM" use HeapError's Display ("out of
memory") instead of its Debug name. −3.4 KB raw, −1.25 KB gzip. What stays on stable:
core's own panics (indexing, `expect`, RefCell) build `fmt::Arguments` with integer
Display impls at their call sites, and JS-visible messages use `format!`, so
core::fmt::write, integer formatting and `format_inner` remain reachable (~9 KB raw).
Removing them would take a non-formatting replacement for ~100 `format!` sites (≈2–3 KB
gzip more).
