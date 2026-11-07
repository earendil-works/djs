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

## D2337: The decomposition stream in two parts with per-position prediction
date: 2026-11-07 · by: bloat · issue: 0131
norm_tables.rs's decompositions were one varint stream (20,066 bytes, 11.6 KB gzip, the
largest Unicode table). They are now two: DECOMP_ENTRIES (gaps, headers, run counts) and
DECOMP_TARGETS (the mapped code points), indexed together every 32 entries, and each
mapped code point is predicted by the previous entry's code point at the same position
(the base letter by the previous base letter, the combining mark by the previous mark),
not by the code point before it: `A + U+0300` then `A + U+0301` cost a byte each.
Together with D2338: module 460,874 -> 455,263 bytes gzip (−5.6 KB). Decoding works in
place in one buffer with a one-byte varint fast path: NFD-heavy work runs 15% faster
than before (wmeasure: 2,746 -> 2,320 M instructions), NFC/localeCompare 12%.
Considered: the CJK compatibility ideographs and Arabic presentation forms as their own
codings (QuickJS has special types), smaller wins than the prediction.

## D2338: Primary composites are derived at run time and cached per heap
date: 2026-11-07 · by: bloat · issue: 0131
The composition tables (COMPOSE_KEYS, COMPOSE_DELTAS, COMPOSE_SECONDS: 6 KB raw, 3 KB
gzip) are gone. The primary composites are the canonical two-code-point decompositions
that are not non-starter decompositions and not in COMPOSE_EXCLUDED (81 code points:
the script-specific and post-composition-version exclusions; the generator asserts that
this rule reproduces Full_Composition_Exclusion). `normalize::Compositions` decodes them
once, at the first composition a VM does, into a sorted `Vec<u64>` (`first << 42 |
second << 21 | composite`, ~940 entries, 7.5 KB) held by the heap
(`Heap::compositions`, Rust memory, not part of the image: rebuilt after a restore), so a
lookup stays a binary search, after an early exit for second code points below U+0300.
`normalize` takes the cache as a parameter.

## D2053: A page allocator under dlmalloc in djs-wasm: huge blocks grow in place
date: 2026-11-03 · by: heap · issue: 0128
On wasm32, djs-wasm's global allocator is `allocator::PageAlloc`, over the module's
linear memory in 64 KiB pages.
- **The page manager (`Pages`)** owns all memory growth. It keeps a sorted, coalesced
  free list of page runs: a fixed array of 64, since an allocator cannot allocate for
  itself; a run freed while the list is full is leaked.
- **Huge blocks** (≥ 256 KiB: in practice the arena and big strings) are page runs of
  their own. A huge realloc:
  1. grows in place into a free run after it, or at memory's end by `memory.grow`;
  2. otherwise moves to the best-fitting free run;
  3. only if there is none, moves to memory's end.
- **Everything smaller** goes to dlmalloc, the same allocator as before, whose system
  allocator now takes pages from the manager instead of growing memory itself. It keeps
  its segments (no release), as before.

Why: before, every growth of the arena that dlmalloc could not do in place copied it,
with the old and new blocks coexisting. Those moments were most peaks, and they were a
chaotic function of sizes (D2047).

**Peak wasm memory**, mean of 24 perturbed runs (D2048), main → branch, MiB:

| workload | main | branch |
| --- | ---: | ---: |
| strbuild | 38.6 | 19.5 |
| arrays | 28.0 | 14.3 |
| higher-order | 28.9 | 15.5 |
| foreach, forof | 9.3 | 5.3 |
| strings | 11.97 | 11.26 |
| json-date | 6.12 | 4.81 |
| json-indent | 6.30 | 5.92 |
| pi-like | 3.70 | 2.81 |

Small heaps are unchanged at 1.6, and the pi bench's warm and cold starts too
(1.94–2.13 MiB). Worse:
- json 6.2 → 6.8 and json-replacer 4.43 → 4.83 (its max 8.0 → 5.6). One of dlmalloc's
  first segments lands right after the young arena and blocks its first in-place growth.
  The arena moves once, and the pages it leaves are only partly reused.
- jsonh 6.2 → 6.3, json-text 2.5 → 2.6, props 2.43 → 2.53.

**Other measurements:**
- Wasm instructions: strbuild −1.8%, higher-order and arrays −0.5%, the rest within
  ±0.8% (noise).
- Code: +1.2 KB wasm, +588 B gzip, within the size ratchet.

**Unsafe:** confined to djs-wasm's allocator.rs. It is the `GlobalAlloc` contract
forwarded to dlmalloc, raw page addresses, and dlmalloc's system allocator reaching the
manager through a pointer. The pointer is set once, when the static allocator is first
used, and the static never moves. djs-core stays safe.

**Tests** (natively, over a fake memory):
- the free list;
- in-place extension;
- moves into free runs;
- a randomized stress test: 3000 operations over small, medium and huge blocks with
  random alignments, contents and overlap checked;
- an arena-like growth to 48 MiB staying near its size.

**Measured and rejected** (each moved single workloads by 0.5–10 MiB either way, like
D2047):
- free pages left before huge blocks placed at the end, for dlmalloc's growth (2–4
  pages, or 1/8);
- dlmalloc granularity 256 KiB / 1 MiB (small runs +0.2–0.9 MiB);
- a minimum of 2–8 pages per dlmalloc growth;
- huge thresholds of 128 KiB to 1 MiB (no change below 1 MiB, worse at it);
- moving a blocked huge block down into the free run before it (no change);
- placing huge blocks only at memory's end (worse);
- letting dlmalloc release segments (no change, +274 B gzip).

## D2054: VMs from the base keep their initial arena room; json's +0.6 MiB is accepted
date: 2026-11-03 · by: heap · issue: 0132
D2053 made json 6.2 → 6.8 MiB and json-replacer 4.43 → 4.83 MiB, means of 24 perturbed
runs. Nothing is changed for it.
- **The cause**, from a log of the page manager over all workloads:
  1. Every growing workload starts the same way. dlmalloc takes 1 page (page 8), the
     decoded base 5 pages (9..14), then the arena 11 pages (14..25), already sized to
     its first collection threshold plus slack (D1990).
  2. When the script compiles, dlmalloc asks for 2 more pages. They land after the
     arena (25..27), so the arena's next growth is blocked and it moves once, to page 27.
  3. The arena then grows in place to the end. Below it stay the old arena's 11 pages
     (partly reused by dlmalloc and a temporary side table) and the 2-page segment.

  json's arena ends at page 27 + 82. Under dlmalloc alone it moved many times, and its
  last move went down into a big coalesced hole at page ~14. Those 13 pages are the
  difference.
- **More initial room doesn't help.** json grows past its first threshold in one native
  operation (11 → 17 pages) before any safepoint. Room for that up front costs every VM
  the pages, including the small ones at 1.6 MiB, and dlmalloc's segment would still
  land after the arena and block a later, bigger growth.
- **A dlmalloc-only pool at instance start doesn't help.** Single runs, MiB:

  | pool | json | small VMs | elsewhere |
  | --- | ---: | ---: | --- |
  | none (D2053) | 6.8 | 1.6 | |
  | 2 pages | 6.8 | 1.6 | identical to no pool |
  | 4 pages | 7.4 | 1.7 | arrays 14.3 → 19.9, higher-order 15.5 → 21.1, pi-like 3.1 → 2.4 |
  | 6 pages | 7.6 | 1.8 | similar mix |

  dlmalloc needs 3–9 pages over a run. A smaller pool only delays the blocking segment
  until the arena is bigger, and a large enough pool costs small VMs 0.2–0.5 MiB.
- **Accepted:** json +0.6 MiB and json-replacer +0.4 MiB, against 2× lower peaks on
  arrays, higher-order and strbuild and pi-like 3.70 → 2.81 MiB.
- **Not built:** letting a collection's shrinking realloc move the arena down into a lower
  free run (0132's follow-up).
## D2339: Messages and formatted strings without core::fmt
date: 2026-11-07 · by: bloat · issue: 0131
`crate::text` (`cat!`, `cat_to!`, `pad`, `hex`) concatenates strings, characters and
integers (zero-padded or hexadecimal) without core::fmt. The ~100 `format!`/`write!`
sites the shipped module reaches use it: the interpreter's and compiler's error
messages, Date's string forms, stack traces, `Function.prototype.toString` of natives,
RegExp source escapes, error summaries. CompileError, json::SyntaxError, WireError and
LoadError get a `text()` method (their Display impls call it; djs-wasm uses `text()`),
HeapError an `as_str()`. Output is byte for byte what `format!` produced (a unit test
compares the integer forms with `format!` over edge values; Date and error-message
tests unchanged). `alloc::fmt::format` leaves the module; what remains of core::fmt
(~6.5 KB raw) is held by core's own panics (`unwrap` on `Result`s with Debug errors,
`assert_eq!`, bounds checks), which stable cannot avoid. Module 455,263 -> 454,583 bytes
gzip (−0.7 KB); Date string formatting runs 9% faster (no Formatter).
## D2264: `djs heap` analyzes dumps through djs-core's heap code; djs-cli reads and writes snapshot containers
date: 2026-11-03 · by: snapshot · issue: 0133
`djs heap <dump>` (crates/djs-cli/src/heap/) reports what is in a raw heap image, a
snapshot container or an older DJSS/DJPI snapshot: a summary (live and garbage, old
region and young), cells by kind and object class, a diff against a reference image,
the largest cells and retained sizes with a path from the roots, strings, shapes,
functions and bytecode, and what a waiting session holds (host calls, promises,
suspended activations with their locals). Text by default, `--json` for scripts,
`--only` for sections; `heap::analyze_image` does the same for an image in memory.

- **No second parser.** The image is loaded with `Heap::from_bytes` (verified), cells
  come from `Heap::cells`, references from `Heap::trace_cell` (the layout table decides
  what a reference is), and every path label is a field name of the layout table, a
  property key from the object's shape or table, or a local name from the template.
  Liveness follows the collector's rules: ephemeron pairs live through their key, value
  stack slots above `SP` do not count. Retained sizes come from a dominator tree
  (Cooper, Harvey and Kennedy) over the live graph.
- **The diff** matches the image's cells to the reference's by a parallel breadth-first
  walk from both roots cells, pairing the targets of the same slot. Immutable cells
  (strings, bytecode, BigInts) pair only with the same content, a shape's transition list
  pairs its children by the key they add (an inserted transition shifts the rest), and a
  cell is unchanged when its bytes match with references compared through the pairing.
  It reports unchanged cells in place and moved, changed cells (differing 8-byte words),
  new cells grouped into subtrees with the new bytes only they keep alive, and the
  reference's cells that are gone: what a snapshot compressed against the reference has
  to encode.
- **Containers in Rust.** crates/djs-cli/src/container.rs reads and writes the layout
  of js/src/snapshot.ts (D2260) and the formats before it, with the `zstd` crate:
  compression at level 3 with the reference as a raw-content dictionary, a window covering
  both and a content checksum. The C library is the decoder that takes raw-content
  dictionaries (the pure Rust ruzstd only takes zstd-format ones); djs-cli is a std tool
  crate, and the shipped module is unaffected. A container against the base uses this
  build's `djs_prelude::BASE` when the hash matches; others need their reference given:
  `node scripts/heap/reference.ts base|pi-prelude` writes a module's base or pi's prelude
  image for a session's start data (`preludeImage` of js/pi/vm.ts). Checked against the
  JS host in both directions by hand (a Rust container restores in `Sandbox.fromBytes`,
  JS containers read in Rust); the Rust tests cover the layout, the older formats and the
  refusals.

## D2055: The old-region walk uses per-kind slot plans
date: 2026-11-03 · by: heap · issue: 0134
A minor collection's walk over the old region (D2041) no longer calls
`walk::for_each_slot` with a closure per traced slot. In wasm that closure was not
inlined, so the walk paid one call per slot (17,110 slots in the base).
- **How:** `compactor::PLANS` is a `static` table indexed by kind byte (28 entries, 16 B
  each), computed at compile time by a `const fn` from `cells::LAYOUTS`. Each entry
  holds:
  - the fixed `Value`/`Ref` fields: byte offsets and a bit mask of which are `Ref`s;
  - the tail element's traced fields, with the tail offset and element size.
- **The walk:** for each slot, a load, a compare against B and, for a `Value`, the tag
  check. Only a young target calls the out-of-line `old_hit`, which marks and remembers
  it. Arrays of values (`ValueArray`, `PropValues`, object slots) get a word-at-a-time
  loop.
- **Tested:** `slot_plans_match_the_layouts` checks every kind and size against
  `for_each_slot`.
- **Wasm instructions:** pi-like 8064 → 7795 M (−3.3%), props 8154 → 7882 M (−3.3%),
  strings −0.8%, jsonh −1.6%, the rest within ±1% (noise).
- **Size:** +459 B gzip, +1.0 KB raw. A first version with a 256-entry table of wider
  entries cost +16 KB raw / +621 B gzip.

## D2056: The collection threshold stays at twice the live size
date: 2026-11-03 · by: heap · issue: 0134
Measured and not adopted: a growth sized from the young part as well. The young part is
`after − OLD_END`, and B = 0 heaps were unchanged. The rule was
`growth = max(live, MIN_GROWTH) + extra`, measured on top of D2055 with two choices of
`extra`:
- **extra = young survivors** (k = 1). Wasm instructions: pi-like −1.9%, json −6.6%,
  jsonh −6.2%, json-replacer −4.5%, json-indent −2.3%, higher-order −1.5%. Peak memory,
  mean of 24 perturbed runs (D2048), MiB:
  - pi-like 2.81 → 3.30, json 6.8 → 7.4, jsonh 6.3 → 8.1;
  - json-replacer 4.83 → 7.70, json-indent 5.83 → 7.70;
  - arrays 14.2 → 17.8, higher-order 15.4 → 21.5, strbuild 19.4 → 22.1.

  k = 2 and 3 gained more and cost more again.
- **extra = min(young, MIN_GROWTH):** pi-like −2.3%, json −3.7%, jsonh −3.0%,
  json-replacer −3.2%, json-date −3.0%. Peaks (24 runs, mean):
  - pi-like 2.81 → 3.30 (+17%), json 6.8 → 7.1, jsonh 6.3 → 6.6;
  - json-date 4.80 → 5.20, json-indent 5.83 → 6.30, json-replacer 4.83 → 5.71;
  - props 2.43 → 2.60, strings 11.25 → 13.09;
  - strbuild mean 19.4 → 20.8 (max 32.7).

Every gain costs peak memory roughly in proportion: collections are fewer only because
the arena grows further between them. pi runs many VMs and memory was the target of
D2041–D2053, so 2–4% of instructions don't buy +0.3–0.9 MiB per VM. Reopen if
instructions matter more than per-VM memory.
## D2340: Heap views show the program's own cells apart from the base realm
## D2340: Heap views show the program's own cells apart from the base realm; REPL dumps are containers
date: 2026-11-07 · by: bloat · issue: 0135
A fresh VM is a copy of the base snapshot: about 5,800 cells and 320 KB of builtins,
their bytecode, templates and names (0135). Every heap view used to report the arena as
a whole, so a three-line example showed "almost 6,000 cells". The docs' Heap tab now shows
the cells allocated after the base (offsets from the fresh VM's length on), by kind, and
names the base realm's cells and bytes separately, with the total; a base cell the
program grew (the atom table, the global object's properties) is reallocated after the
base and counts as the program's. After *Collect now* the compaction slides cells, so the
split is by the same offset and approximate.
The REPL's `.snapshot` writes a snapshot container (app 3, `repl`, metadata `{}`)
compressed against the base image by default, through djs-cli's container module (0133):
a small session is ~16 KB instead of its ~330 KB image; `.snapshot --raw` writes the
image. `.restore` reads containers of any app (decompressed with this build's base) and
raw images. `.heap` runs `djs heap`'s analyzer over the live VM against the base image:
by default the summary and the diff, which lists the session's new cells and the base
cells it changed apart from the base realm.

## D2300: JSON strings: quoting jumps to the first byte to escape; parsing skips plain words after an escape
date: 2026-11-08 · by: perf · issue: 0129
Codemode JSON carries texts: file contents with an escaped newline every line or so.
- Stringify's `quote` already skipped words with no byte to escape (0116). On a word that
  has one, it now jumps straight to the first such byte, the lowest flag of the SWAR
  mask, instead of going byte by byte up to it. The mask (`json::escape_mask8`) is
  shared with the parser.
- Parse's `scan_string` keeps 0116's byte loop for strings without escapes (keys, short
  values). At the first escape (or UTF-8 multi-byte sequence) the rest of the string
  goes to an out-of-line `scan_rest`. After each escape it skips plain bytes 8 at a time
  (`plain_run`).
  - A word test inside the loop itself, or a `plain_run` call in its escape arm, cost
    short-string parsing 1–2% (wasm json +1.3%), which is why the split exists.
  - One-byte and UTF-8 input share one `scan_rest` instance (a runtime `utf8` flag).
    Two-byte input, which is rare, takes `scan_rest` from the start: one loop, a
    smaller module.

Tried and dropped: decoding escaped one-byte strings into a scratch buffer during the
scan, then copying them once (the one-pass idea from 0116). Natively json-text was +2.6%:
the scratch copy plus the cell copy is the same traffic as the second pass, plus a Vec.

wasm (with D2301): json-text −12%, others ±1%.

## D2301: slice, join, split and CreateDataPropertyOrThrow on plain arrays in one step
date: 2026-11-08 · by: perf · issue: 0129
pi-like's `LINES.slice(0, n).join("\n")`, `text.split("\n").filter(..)` and map/filter
results, profiled in wasm:
- `slice` (@fast): when ArraySpeciesCreate would be a plain ArrayCreate (`plain_species`:
  no own `constructor`, untouched %Array% and @@species), the result is
  `Heap::new_array_from_dense`. That is one allocation and one copy of the dense range;
  a hole declines. Before, it was a Vec of the values plus a [[Set]] per element, each
  walking the prototype chain and growing the store. Anything else declines to the
  builtin, whose path (ArraySpeciesCreate, `%ArraySliceDense`) is what the fast path did
  after that check.
- `join` of flat one-byte strings, `undefined`/`null` and a flat one-byte separator:
  `Heap::join_flat_latin1` sizes the result in one pass with one kind read per element,
  then copies from the cells. There is no list of pieces and no per-piece view. Ropes and
  two-byte strings keep the pieces path. A leaner variant (a kind check inside
  `string::join`) saved 232 B gzip but cost pi-like 2.3%.
- CreateDataPropertyOrThrow into an Array (`Heap::create_element_fast`: native
  `%CreateDataPropertyOrThrow` and the intrinsic, and `%ArraySliceDense`) defines the
  element without the prototype walk [[Set]] needs. An Array.prototype index setter is
  never called, as the spec says. Non-extensible arrays, index properties and a fixed
  `length` decline.
- `split` with a one-byte separator copies each piece with `alloc_latin1_pieces`
  directly. Empty pieces share the empty atom, the vectors are sized, and the whole
  string is returned as itself.

Natively, `LINES.slice(0, 20 + n).join("\n")` went 453 → 189 Minstr. wasm results with
D2300 are in D2302.

## D2302: Size trade: +1.4 KB gzip for pi-like −12.7%
date: 2026-11-08 · by: perf · issue: 0129
D2300 and D2301 add 1,359 bytes to the shipped module's gzip (455,584 → 456,943 on main
1954f0dc; +0.3%). The coordinator accepted the trade. `tests/scores/size.tsv`'s
ceiling is raised under this decision to 457,967 (the new size plus the usual 1,024
slack).

wasm instructions (n=3, spread ≤0.7%) against the main they were measured on:

| workload | before | after | change |
| --- | ---: | ---: | ---: |
| pi-like | 7815 | 6825 | −12.7% (0.94x quickjs-wasi, was 1.08x) |
| json-text | 1810 | 1599 | −11.7% |
| higher-order | 12715 | 12352 | −2.9% |
| strings, strbuild | | | −2% |

The other workloads stayed within ±1%. pushpop moves between ~4205 and ~4305 from run to
run, main included.

Already removed before the trade:
- shared `scan_rest`/`plain_run` instances;
- slice declining to the builtin instead of keeping its own species fallback.

Not taken for this trade: pruning more @fast entries (toFixed, lastIndexOf,
fromCharCode, startsWith/endsWith).

Measured for the owner, not done: inlining the native API in wasm (D1634).
- `n_slot` and `n_arg`: +4.6 KB gzip, for higher-order −10.5%, foreach −5.7%, pi-like
  −2.1%.
- The full accessor set: +10.7 KB, for −17%, −9%, −2.6%.
## D2057: The cells a VM replaces first sit at the end of base and sealed images
date: 2026-11-03 · by: heap · issue: 0136
**The mechanism**, from `djs heap --diff base` on a fresh REPL's `.snapshot`:
- The REPL, `Sandbox.bytes()` and pi's `snapshot()` all collect in full (D1890), and a
  full collection compacts the old region too: it keeps only the region's live cells
  (D2041). Minor collections leave the old region alone, but these dumps come from full
  collections.
- A fresh VM replaces some base cells outright:
  - the atom table's entries array, 30.3 KB at offset ~68 KB, copied by the first new
    atom;
  - the global object's property table and index (4 KB), copied by the first global;
  - the root shape's transition list.

  The full collection drops the dead originals and slides everything after them down.
  Only 14.7% of the image's words stayed at the reference's offsets, and an empty
  Sandbox's container was 15.7 KB, a fresh REPL's 16.0 KB.
- pi's prelude image had the same problem with the prelude's `pending` Map (snapshot,
  0133).

**The fix:**
- `Heap::first_grown(global)` lists, in this order:
  1. every Map/Set's entries and index;
  2. the root shape's transitions;
  3. the global object's property index and table;
  4. the atom index;
  5. last and largest, the atom entries, which the first new atom replaces.

  A dying cell shifts only those after it.
- The base build (`Vm::prepare_base_image`) passes these cells to `order_for_image`, which
  puts them after all others (the kind grouping of D2251 stays for the rest).
- `Vm::seal` first copies them to the end of the arena (`Heap::move_to_end`, which
  repoints the slots; a Map/Set's old entries forward to the copy, for live iterators).
  The seal's full collection then drops the originals and leaves the copies at the end
  of the sealed image.
- The atom entries grow by a quarter instead of doubling (`push_atom_entry`): the base's
  ~2,000 atoms doubled to 3,900 slots on the first new atom, 15 KB of zeros in every
  image.

**Measured** (scripts/snapshot-research/wasm.ts and the REPL; main → branch):
- Empty Sandbox container 15.7 → 0.8 KB, with 1000 records 31.2 → 15.7 KB.
- Fresh REPL `.snapshot` 16,023 → 835 B, and after `let x = 1` 16,200 → 1,005 B.
- Raw fresh REPL dump 340,784 → 329,208 B. The words at the reference's offsets went from
  14.7% to 90.3%; the rest is the session's own changes.
- pi waiting sessions (medians): agent-loop 7.0 → 6.2 KB, sequential 7.4 → 6.7 KB;
  ask, records, big-log and tool-heavy are equal within 1 KB. Collected images are
  11 KB smaller.
- `vm_new` 3.9–4.0 µs both; the pi bench's starts are unchanged.
- Determinism: native-vs-wasm, e2e, difftest and the base built twice all agree.

## D2058: The size ceiling rises by 600 B for stable snapshot layouts
date: 2026-11-03 · by: heap · issue: 0136
D2057's shipped code costs +588 B gzip: `seal`'s relocation, the list of first-grown
cells, and the atom table's quarter growth. The kind-ordered base costs +40 B of data. The
module went from 455,584 to 456,172 B gzip, over the ratchet ceiling (455,619). On top
of D2302's ceiling (457,967), which landed meanwhile, it rises by 600 B to 458,567 in
tests/scores/size.tsv.
- **Bought:** empty Sandbox snapshots 15.7 → 0.8 KB, fresh REPL dumps 16 → 0.8 KB, and
  11 KB smaller raw images of every VM from the base.
- **Not found:** no cheaper encoding. The relocation was already cut from +2.1 KB raw to
  +1.4 KB (header walk instead of the cell iterator, one push site).

## D2060: The base is decoded or injected straight into the first VM's arena
date: 2026-11-03 · by: heap · issue: 0139
Before, a wasm instance decoded the base, or received it from the host, into a buffer it
kept for its lifetime (D2252), and every `vm_new` copied that buffer into a new arena.
With one VM per instance (every Sandbox and pi session), the kept copy (322 KB, 5 pages)
was used once.
- **The first VM takes the bytes.** `vm_new` and `vm_new_reserved` take the instance's
  decoded or injected base as their arena (`Vm::from_base_vec`,
  `djs_prelude::new_vm_owned`). The buffer grows in place to the arena's capacity: the
  page allocator extends a huge block at memory's end (D2053).
  - Without a base, `vm_new` decodes straight into a buffer with the arena's capacity
    (`djs_lzrc::decode_into`).
  - Injected and `djs_base_ptr`-decoded buffers are exactly the base's size, so an
    instance that only reads the base (restoring a snapshot against it) holds no arena
    room.
- **Later VMs.** After the first VM has taken the base, the next `vm_new` or
  `djs_base_ptr` decodes it again and the instance keeps it; further VMs copy it, as
  before. A multi-VM instance decodes twice instead of once; `vm_new` in such an instance
  stays a 3.8–3.9 µs copy.
- **The ABI is additive; one lifetime changes.** `djs_base_ptr`'s pointer is valid until
  the next `vm_new`, no longer for the instance's lifetime. A host that reads it after
  `vm_new` gets a fresh decode, which still works but costs 5 ms and the memory again.
  js/src/sandbox.ts and js/pi/engine.ts call `rememberBase` before `vm_new`, and inject
  only into instances that create a VM; pi's restore instances no longer receive a
  base they never use.
- **Verification is unchanged:** the length and hash of injected bytes. Tested natively
  (decode, inject and corrupted injection into the arena; second and third VMs) and in
  wasm (js/test/base.test.ts: the arena's address is the base's).
- **Measured** (release module, main → branch):
  - empty Sandbox 1.50 → 1.19 MiB, first or later instance;
  - pi warm `return 1` 1.81 → 1.50 MiB, cold 2.00 → 1.69, tool-heavy warm 1.94 → 1.63,
    cold 2.00 → 1.69;
  - all 22 perf workloads −0.3 MiB in their 24-run means (D2048), e.g. pi-like
    2.98 → 2.66, json 6.61 → 6.30;
  - first `Sandbox.create` 12.5–12.9 ms both; later ones ~20 µs faster (no copy);
  - pi warm start 0.52 → 0.43 ms (no injection into restore instances);
  - code +157 B gzip.
## D2303: Callback loops in one intrinsic, with a reused frame (`%EachDense`, the `Each` continuation)
date: 2026-11-08 · by: perf · issue: 0137
The dense part of the loops of forEach, map, filter, some, every, find, findIndex and
(forward) reduce is one reentrant intrinsic:
`%EachDense(mode, o, k, len, f, thisArg, a)` (builtins/intrinsics.d.ts,
interp/run/each.rs).

How it works:
- Its arguments, plus a kValue slot, stay on the stack as the loop's state.
- The callback gets one frame, pushed with a new continuation `Each` (frame kind 7).
- When the callback returns (in the hot loop or in `do_return`), `each_return`
  completes the step: map's store, filter's keep, a decision, reduce's accumulator.
  It then rewrites the same frame for the next element and starts it at pc 0: the
  arguments padded with undefined, the header's NEW_TARGET, undefined locals. Or it
  pops the frame and gives the builtin `k` (or -1 - k for a decision).
- Each element is the spec's step: a present dense element of an Array, so HasProperty
  and Get observe nothing. `len` is fixed, the element is re-checked every time, and
  the builtin's own loop takes over from `k` at the first hole, shorter array or other
  object.
- Only plain closures qualify (the `n_call_closure` rule).
- An async callback's frame is replaced on return (`tail_call_builtin`); it is entered
  fresh rather than reused.

The store targets, which never run user code:
- `a` must be an Array: map's result, filter's result, find's box, reduce's box.
- filter needs a plain species (so `a` is fresh and its length counts the kept
  elements).
- reduce passes the accumulator in and out as element 0 of its own rest array.
- At the heap cap a store collects and retries once (D1900).

Fuel, safepoints and stack contents are the same in the hot and general paths
(`hot_loop_is_invisible`). Heap images see ordinary frames and values.

These builtins, and Reduce, FindViaPredicate and CollectPresent (now one
`%ArraySliceDense` copy for a dense array), are no longer `@native`. Their Rust state
machines cost 8.2 KB raw.

Per-call costs of the bytecode versions:
- LengthOfArrayLike is `@fast`: an Array's own length.
- The hot loop answers `%ToObject` of an object and `%IsCallable` inline.
- The four callback builtins check callability inline instead of calling
  RequireCallable.

Measured in wasm against main 5d66ef74:
- The module is 196 B smaller in gzip: 457,604 → 457,408.
- Workloads (n=3, rechecked at n=5): pi-like −7.7% (0.87x quickjs-wasi), foreach −27.5%
  (0.99x), higher-order −30.9% (2.58x → 1.78x). Every other workload is within ±0.5%.
- Per element, from main to now (quickjs-wasi in brackets):

  | builtin | main | now (quickjs-wasi) |
  | --- | ---: | ---: |
  | forEach | 2176 | 1156 (1007) |
  | map | 2487 | 1265 (1052) |
  | filter | 2764 | 1379 (1019) |
  | reduce | 2070 | 1392 (869) |
  | some/every | 3714 | 1000 (785) |
  | find | 2203 | 1000 (796) |

- The cost: a call on an empty or one-element array is 0.8–3.8k instructions dearer than
  the native builtin was. For forEach that is +1.9k (+40%), filter +3.5k, reduce
  +3.8k: the bytecode builtin's intrinsic calls through the general path. Arrays of a
  few elements already come out ahead.

The native-API inlining trade (D2302's numbers), re-measured on top of this:
- n_slot and n_arg: +2.9 KB gzip, for higher-order −4.8% (sort's comparator) and
  json-replacer −2.7%; pi-like −0.4%, foreach 0.
- The whole set: +6.5 KB, for −8.7% and −3.4%; pi-like +0.4%.

With the callback loops out of the natives, the trade no longer pays; I recommend
dropping it.

## D2350: The portable-snapshot prototype lives outside the workspace and builds against older trees
date: 2026-11-02 · by: portable · issue: 0141
`scripts/snapshot-research/portable` is a standalone crate (like snaplab), not part of the
workspace or the gate, using only djs-core's public heap API. To measure a transfer
between builds, the same sources are copied into a `git archive` of the older commit and
built there (`--features old` selects that tree's one changed record schema). The
design it supports, docs/portable-snapshots.md, is a proposal; images stay build-pinned
(D2260) and replay remains the planned cross-version record (0140) until the owner
decides.

## D2351: A snapshot importer never holds cell offsets across a VM run
date: 2026-11-02 · by: portable · issue: 0142
Rebuilding state in a VM sometimes needs JavaScript to run (re-issuing host calls, a
prelude's `adopt(id)`, subscribing `run` again), and a run can reach a safepoint that
collects and moves cells. The 0142 prototype crashed in three of six cases until every
cell it obtained before or between runs was held by a handle and turned into an offset
only after the last run; building the graph itself allocates but never collects (ARCH
"Allocation never collects"). 0141's importer had the same latent bug in its host-call
re-issue loop (fixed). Any engine-side importer follows the same order: runs first,
offsets after, then build without running.

## D022: Frozen builtins as a runtime VM option, plus an immutable builtin prefix
date: 2026-11-02 · by: owner · issue: 0145, 0146, 0147
After docs/frozen-builtins.md (0144): frozen builtins become a VM option at creation
(region-based: builtin objects below the prefix boundary behave as frozen, with the
override fix), not a separate build profile; the default stays spec-conformant. The base
image becomes an immutable prefix of every arena, and images, snapshots, canonical_hash,
collections and determinism cover only the session region. A stable, append-only builtin
registry names builtins across builds. All three phases run in parallel. pi-private is
not changed now (pi dropping its lockdown comes later).

## D2062: `Roots::BASE_END` marks the builtin prefix
date: 2026-11-08 · by: heap · issue: 0147
The first step of D022's immutable prefix, merged early for 0146.
- **The field:** `Roots::BASE_END` (offset 80; Roots' minimum size is 88) is the end of the
  base image in a VM's arena.
  - `Vm::prepare_base_image` sets it to the image's length, so every VM made from the
    base (`vm_new`, restored images) starts with it.
  - 0 in a heap without a base (`Heap::new`, `Vm::new` before `prepare_base_image`, the
    canonical copy).
  - `Heap::base_end()` reads it.
- **Distinct from `OLD_END`,** which also covers what an embedder sealed (pi's prelude,
  D2041).
- **Until the prefix is immutable** (0147's later steps), full collections forward it
  like `OLD_END`, to the live bytes below it, so it stays a cell boundary that separates
  base cells from session cells. Minor collections leave it alone. Canonical (Cheney)
  collections set it to 0.
- **Checks:** `Heap::verify` requires a cell boundary. `release_top` lowers it like
  `OLD_END`.
## D2371: The builtin registry: stable ids by spec path, append-only, gate-checked
date: 2026-11-07 · by: bloat · issue: 0145
Every builtin object, function and symbol of the base realm and every intrinsic has a
stable id in tests/builtins/registry.tsv (D022 phase 0, docs/frozen-builtins.md):
- **Keys:** `value` entries are paths in the spec's style from `globalThis`, a global,
  a realm intrinsic no global reaches (`%TypedArray%`, `%ArrayIteratorPrototype%`) or a
  prelude function registered for an intrinsic (`%intrinsic:Name%`), through properties
  (`.key`, `["key"]`, `[Symbol.iterator]`), `get `/`set ` for accessors, `.[[Prototype]]`
  and closure captures (`~N`, for the 220 prelude helpers that builtins capture and no
  property reaches). `intrinsic` entries name the intrinsics table by name: the kinds of
  builtin closures and the operands of `call_intrinsic`. New builtins get the first path a
  breadth-first walk finds (globals by property, then realm roots, then registered prelude
  functions, then `[[Prototype]]` links, then captures; keys sorted), so the spec's names
  win. Only two realm-internal objects (the global lexical record and the registries,
  per-VM state) have none, and a test says so.
- **Append-only:** ids are 0, 1, 2, ... in file order and never change; a builtin that goes
  away keeps its line with `removed: <reason>`; a second path to a named object is an
  alias (the object keeps its lowest id). `scripts/registry-update.sh` appends and
  tombstones (`cargo run -p djs-prelude --example registry -- update|check`).
- **Checks:** a djs-prelude test fails when a builtin or intrinsic has no live entry or a
  live entry no longer resolves; `scripts/registry.py check main` (a gate step) fails when
  a line of main's table is renumbered, dropped or rewritten. The file merges with git's
  union driver, and scripts/regen.sh (`scripts/registry.py merge`, then the update)
  renumbers a merged tail after main's entries.
- **API:** djs_core::vm::registry (format, paths, `check`, `update`, `name_cells(heap)`:
  resolves the paths on any heap); djs_prelude::registry (feature `registry`, on except in
  djs-wasm): the committed table, `path`, `id_of_path`, `id_of_intrinsic`, and
  `base_offset`/`id_of_base_offset` from a table build.rs computes on the base image.
  `djs heap` names builtins by their registry paths (`%Array.prototype.map%`).
- **Limits:** capture paths change with the builtin's code (tombstone and re-add). A path
  resolved on a session heap follows whatever is there: a script that replaced
  `Array.prototype.map` gets its function named so, and pi's lockdown turns some data
  properties into accessors, which then resolve under `get `/`set ` only. With the
  immutable prefix (D022, 0147) the base offset table names prefix cells exactly.

## D2063: The base image ends with a tail of per-VM cells; nothing writes below BASE_END
date: 2026-11-08 · by: heap · issue: 0147
The second step of D022's immutable prefix. `BASE_END` (D2062) now splits the base image
into the builtin prefix and a **tail** of the cells a VM writes. `Vm::prepare_base_image`
puts the tail last (`order_for_image`) and sets `BASE_END` where it starts.
- **The tail**, in order:
  1. the realm and `Math.random`'s state;
  2. the global object;
  3. the global lexical record and `Symbol.for`'s registry, with their property storage;
  4. Map/Set tables (D2057);
  5. the global object's property table and index;
  6. the atom table cell, its index and its entries;
  7. the host call table, the interpreter cell, the value stack.
- **Nothing is written lazily into the prefix:**
  - Every builtin object is marked `OBJ_IS_PROTO` at build (D1621), instead of when an
    inline cache first caches through it. The tail's own objects are not marked: adding
    a global must not bump the prototype epoch.
  - Header flags are written only when they change.
  - String hashes are not cached for strings below `BASE_END`. They are recomputed
    instead: the few hashed at run time are cheaper than 3.6 KB of incompressible hashes
    in the module's coded base (+1.6 KB gzip, measured).
- **Root shape:** `Roots::ROOT_SHAPE` is cleared in the image. A VM grows its own shape tree
  from a root created on first use. The builtins' shapes keep the old root, so session
  objects never extend a builtin's transition list or descriptor array.
- **Checked:**
  - a write counter over the arena and hot-loop stores below `BASE_END` (roots cell
    excluded), run over all 300 harvested codemode cases and the codemode corpus,
    counted 0 writes. Before, it counted 1.2 M writes into 280 words;
  - a djs-prelude test checks that no prefix cell references the tail.
- **Measured:** wasm instructions within ±0.5% except props −1.2%. Module +60 B gzip. The
  base keeps its size; the tail is 25 KB of it.

## D2304: Frozen builtins: gates in the property layer, the override fix in [[Set]]
date: 2026-11-08 · by: perf · issue: 0146
D022's runtime option, `VmOptions::frozen_builtins` (wasm `vm_new_with` flag 1, Sandbox
and js/pi `frozenBuiltins`).
- **State:** the `Roots::FLAGS` bit `ROOT_FROZEN_BUILTINS` (part of the image, so a
  restored VM keeps its mode) and `Roots::BASE_END` (D2062). The heap caches the boundary
  (`frozen_end`: BASE_END with the bit, else 0) and refreshes it on load, after every
  collection and in `release_top`. `Heap::frozen(o)` is `o < frozen_end` plus, below the
  boundary only, the exemptions; flag off it is one compare against 0.
- **Exempt:** per-VM objects in the prefix: the global object, the global lexical record,
  the `Symbol.for` registry and the constructors table (`realm::FROZEN_EXEMPT`), until
  0147 moves them out. The global object stays fully ordinary (its builtin bindings are
  writable and configurable, as in the spec and under SES's lockdown).
- **Gates, all in heap/object.rs:** `is_extensible` false; `prevent_extensions` true
  without a write; `read_loc` (so `get_own_property`, `lookup_property` and everything
  above) reports data properties non-writable and every property non-configurable;
  `define_own_property` validates against that view and never writes; `delete_own_property`
  fails for existing keys; the fast writers (`prop_set_value`, `shaped_set`,
  `shaped_add`, `set_own_data_value`, `set_element_fast`, `create_element_fast`,
  `array_splice_dense`, `array_can_pop_dense`) decline. `shaped_set` needs its own check:
  an inline cache keyed on a shape can be filled by a session object that shares a
  frozen builtin's shape. `set_prototype_of` follows from `is_extensible`. Internal state
  (object slots, header bits, realm slots, collections) is written as before.
- **Override fix:** in `ordinary_set`, `set_property` and the `super.x = v` path, an
  inherited non-writable data property whose holder is frozen and not the receiver does
  not stop the assignment: the receiver gets an own data property (CreateDataProperty).
  Assigning to the builtin itself, accessors without setters, proxies, `defineProperty`
  and ordinary frozen objects keep the spec's behavior. This is SES's override taming for
  every property, and a deviation from the spec (test262 expects the TypeError), which is
  why the mode is opt-in.
- **Private names:** adding a private field or method to a frozen builtin throws (it is
  not extensible), as the stage-3 "non-extensible applies to private" proposal has it.
- **Collections:** a canonical (Cheney) collection has no prefix, so `Vm::collect_canonical`
  is a full collection with the mode on. `Vm::new` with the option and no base sets
  BASE_END to everything made at creation.
- **Cost (wasm instructions, 22 workloads):** flag off +0.0 to +0.5% against main
  (pushpop's two modes aside), flag on within +0.0 to +0.8% of flag off (props +0.8%:
  global stores take the exemption check). +521 bytes gzip.
- **Known limits:** `Error.stackTraceLimit` and `Error.prepareStackTrace` are frozen with
  `Error` (pi does not set them); a builtin is frozen by where it lives, so the mode needs
  a base prefix (no effect on a heap without BASE_END).
- **pi:** its prelude without the lockdown on the mode starts in 0.40 ms instead of
  2.8–3.1 ms (native, release), and its sealed image is 375.6 KB / 6,362 cells instead of
  479.9 KB / 8,330 (the base is 322.4 KB). pi's prelude *with* its lockdown still runs on
  the mode: its walk finds no writable, configurable property to convert and only freezes
  the global object's bindings, but costs 2.1 ms and leaves 418.3 KB. pi gets the gain by
  turning the option on and dropping the lockdown (pi-private, later).

## D2064: Collections never move or free the builtin prefix
date: 2026-11-08 · by: heap · issue: 0147
Below `Roots::BASE_END`, cells keep their offsets for the VM's whole life.
- **Full collections** (`Heap::collect`) are `mark_compact(BASE_END)`. The prefix is
  walked like an old region (D2041, D2055): its slots are roots, and its cells are
  neither marked nor moved. Everything above it is collected and compacted, pi's sealed
  region included. `BASE_END` no longer changes after `prepare_base_image` (D2062's
  forwarding is gone).
- **Canonical collections** (Cheney, `collect_canonical`) copy the prefix as it is, then
  lay out the cells above it breadth-first from the roots. Keys in the prefix count as
  live for ephemerons. `BASE_END` is kept, so a canonical image of a VM is still a VM of
  the same base, and `canonical_hash` covers the prefix as it is (the same bytes in
  every VM unless a script changed a builtin) plus the canonical session region. A heap
  without a prefix (base 0) is laid out from the roots as before.
- **Frozen builtins:** `Vm::collect_canonical` no longer falls back to a full collection
  (D2304's workaround), and `realm::FROZEN_EXEMPT` is gone: the per-VM objects are above
  the prefix (D2063). `Vm::new` with frozen builtins makes its realm into a base image
  first (`prepare_base_image`), so it has the same prefix and tail as a VM from the base.
- **Trade, default mode only:** a builtin that a script changes in ways that replace its
  storage (adding properties, `defineProperty`) leaves its old storage in the prefix,
  where it is never freed. The same holds for anything only that dead storage reaches.
  This is bounded by how much scripts reshape builtins. With frozen builtins nothing in
  the prefix changes.
  - Measured: pi's JS lockdown reshapes ~500 builtins and leaves 16.5 KB of dead
    PropValues in the prefix of every pi session. Before, the full collection at its
    seal freed them.
  - `djs heap --diff` pairs such unreachable prefix cells by offset, since they never
    move, so they are not reported as new and gone.
- **Measured:** wasm instructions pi-like −1.5%, arrays −0.8%, json −1.1%, the rest ±0.5%
  (full collections no longer mark and slide the base). Module −20 B gzip.

## D2065: A debug write barrier below BASE_END, armed by frozen builtins or one option
date: 2026-11-08 · by: heap · issue: 0147
In debug builds, an armed barrier panics on any write into the builtin prefix (below
`Roots::BASE_END`, past the roots cell).
- **The checks:** `Arena::write`, `slice_mut`, `copy_within` and `split_mut` check each
  write. `Heap::check_prefix` compares the whole prefix with its copy from when the
  barrier was armed. That runs after every `Vm::run` and every collection, for writes
  through raw slices (the hot loop). Collections run with the per-write check
  suspended: they flag and clear header bits.
- **Arming:** frozen builtins (`Roots::FLAGS` `ROOT_FROZEN_BUILTINS`, D2304) always arm
  it. Otherwise one switch does: `VmOptions::prefix_barrier` (`Heap::force_prefix_barrier`).
  It is not image state: it is applied at creation and restore.
- **Release builds** carry only the field and a no-op: no check, no copy.
- **Armed in the gate** (TESTING.md "The prefix write barrier"):
  - perf's vm/frozen_tests.rs, and any VM with frozen builtins;
  - difftest's `codemode/` and `codemode-harvest/` areas in every mode the gate runs
    them.

  e2e, test262 and the rest of the corpus change builtins on purpose, so they run
  unarmed.

## D2066: Session images: the roots cell, a canonical prefix diff and the session region
date: 2026-11-09 · by: heap · issue: 0147
The last step of D022's immutable prefix. A VM made from the base shares its builtin
prefix (`[0, BASE_END)`, D2062–D2064) with every other VM of the build, so a stored image
leaves it out.
- **Format** (`vm::session`, `"DJSs"`, format 1), little-endian:
  - header: magic, format, build hash, `BASE_END`, FNV-1a 64 of the base's bytes
    `0..BASE_END`, the roots cell's size, n;
  - n u32 offsets of the prefix words (past the roots cell) that differ from the base,
    ascending, padded to 8 bytes; their n u64 values;
  - the roots cell, then the image from `BASE_END` on.

  It is a pure function of the raw image and the base, so equal images give equal session
  images: the diff is canonical. `session_image`, `expand_session` (checks the build,
  `BASE_END`, the roots size and the prefix hash, then the offsets: in range, aligned,
  ascending), `session_diff`; `Vm::session_bytes(base)`, `Vm::from_session`.
  js/src/snapshot.ts has the same functions (`sessionImage`, `expandSession`).
- **Containers:** `"DJSN"` format 2 adds byte 13, the image kind: 0 raw, 1 a session
  image against the module's base image, 2 one against the container's reference.
  Writers (js/src/snapshot.ts `encodeSnapshot({ base })`, djs-cli `container::write`)
  take the reference when it has the same prefix, else the base, else store the raw
  image. Against pi's prelude image the diff is what the session changed since the
  prelude. As a diff against the base, the 1,749 words (660 cells) pi's lockdown changes
  cost 8 KB per container, though the dictionary holds them in place. Format 1
  containers and the formats before the container still load; other formats fail with
  "snapshot container format N".
- **Sizes (measured):** compressed containers are unchanged within 2% (the dictionary
  already made the prefix free): a new Sandbox 1,034 B, pi sessions waiting on a tool
  2.1–6.6 KB. Uncompressed ones lose the prefix: a new Sandbox 32 KB instead of 330 KB.
  An unchanged builtin costs nothing, a changed word 12 bytes. JS encode +0.05–0.1 ms
  (one pass over the prefix as 32-bit words; the base's prefix hash is cached).
- **canonical_hash** stays FNV-1a over the canonical copy's raw image. Given the base, the
  raw image and the session image determine each other, and the canonical collection
  leaves the prefix as it is (D2064). Hashing the session image would need the base in
  the VM.
- **Order independence, what holds:** calling builtins leaves no trace in the prefix: no
  prefix writes (D2063) and an empty diff. Calls that return primitives or plain arrays
  leave the same session image whatever order they ran in (test
  `touching_builtins_in_any_order_leaves_the_same_session`).
- **What doesn't, yet:** objects that builtins *create* with named properties (iterator
  results, RegExp match results, property descriptors) get their shapes in the session
  region, in the order they are first created, and shapes are never freed (D1831). So
  `new Map(...)` (+168 B) and `/a/.exec()` (+288 B) leave shapes behind, and two runs
  using them in opposite orders lay the session region out differently. Pre-creating
  those shapes in the base is follow-up work.
- **Analyzer:** `djs heap` has a `prefix` section: the changed prefix words against this
  build's base, by cell, named by registry path (`%Array.prototype%`, `%Math%`).

## D023: Notes superseding parts of D2304 after 0147
date: 2026-11-02 · by: coordinator · issue: 0146, 0147
D2304's "Exempt" and "Collections" points no longer hold: 0147 moved the per-VM objects
(global object, global lexical record, Symbol.for registry, constructors table) above
BASE_END (D2063), so `realm::FROZEN_EXEMPT` is gone and frozen means `o < frozen_end`; and
collections never move or free prefix cells (D2064), so `collect_canonical` no longer falls
back to a full collection in frozen mode.

## D024: No compatibility with unreleased formats
date: 2026-11-02 · by: owner · issue: -
djs has never been released, so there is nothing old to stay compatible with. Snapshot,
image and container formats (and ABI exports) are changed in place: no version bumps
for compatibility, no readers for older formats (DJSN format 1, DJSS, DJPI, raw images in
containers where the session image replaces them), no deprecated exports kept. A format
still carries a version field so a mismatch fails clearly. Revisit at the first release.

## D2400: One snapshot container format, session images only
date: 2026-11-09 · by: heap · issue: 0148
D024 applied to the snapshot files of D2260 and D2066:
- **Container:** `"DJSN"` version 2 is the only one read. Any other version fails with
  "snapshot container format N; this host reads format 2". The number stays at 2 rather
  than going back to 1, so containers written before session images (version 1, raw
  images) fail at the version check instead of deeper.
- **Gone:** the readers of version 1 and of `"DJSS"` (Sandbox) and `"DJPI"` (pi), in
  js/src/snapshot.ts, the Sandbox, js/pi and djs-cli; and raw images inside containers.
- **Byte 13** says only what the session image's prefix words are against: 0 the module's
  base image, 1 the container's reference (`AGAINST_BASE`, `AGAINST_REFERENCE`). Writers
  refuse an image without the base's prefix (a VM not made from the base).
- **Kept:** djs-cli reads a bare raw image (`Vm::bytes`, the REPL's `.snapshot --raw`,
  reference files) or a bare session image as `djs heap` input: those are the image
  itself, not an older container. `baseReference` requires the base exports (every
  module has them).

## D2067: Builtin result shapes and the engine's queues are in the base; the ceiling rises 720 B
date: 2026-11-09 · by: heap · issue: 0148
Completes D2066's order independence: using builtins, in any order, adds nothing to the
session region.
- **Shapes:** `prepare_base_image` gives every VM its own root shape (D2063) with the
  shapes of the objects builtins and the engine create already in its tree
  (`vm::image::BUILTIN_RESULT_SHAPES`, from a survey of builtin calls): iterator results,
  RegExp match results with `groups`/`indices`, data and accessor descriptors, function
  `length`/`name`/`prototype` (writable and not), `constructor`, RegExp `lastIndex`,
  Error `message`/`cause`/`errors`, `Promise.allSettled` results, `Promise.withResolvers`,
  `Proxy.revocable`, template `raw`. The tree goes to the base's tail (writable, above
  `BASE_END`), so a script extending such an object adds its transition there, as for its
  own objects, and nothing needs a side table: in the harvested and codemode corpora 2 of
  310 cases extend a pre-created shape at all. The chains come from the builtin set: a
  build with fewer builtin families (the slim codemode build, 0149) would pre-create
  fewer, and a new builtin that returns objects with named properties adds its chain.
- **Queues:** the job queue (8 jobs), the host inbox (4 results) and the unhandled
  rejection list (8) are allocated in the tail, their slots `undefined`, and are emptied
  instead of dropped (`clear_jobs`, `take_rejections`, ...). Their first use no longer
  places a cell in the session region at a time that depends on the script.
- **Names:** `prepareStackTrace` and `stackTraceLimit`, which making an error interns, are
  kept in the base (interned before the tables are shrunk). Names a compilation interns
  (`<home>`, identifiers, `<fields N>`) remain the script's own.
- **Still session state:** results keyed by data (`getOwnPropertyDescriptors(x)`,
  `Object.groupBy`, `JSON.parse`, regex group names), the FinalizationRegistry list (rare
  in codemode), and queues that grow past their base capacity.
- **Trades, recorded:**
  - module +648 B gzip (458,304 → 458,952): shapes 221 B, descriptor arrays 160 B,
    transition arrays 101 B, names ~130 B, queues 82 B, code 73 B; the base +2.8 KB raw,
    per VM too. The ceiling in tests/scores/size.tsv rises by 720 B to 459,164.
  - props +1.4% wasm instructions (10 runs × 2; noise ±0.7%; the rest within ±0.9%,
    pi-like +0.1%): the root shape has 13 more children, and `shape_without` (a delete in
    the middle of a shape-mode object) binary-searches the root's transitions on every
    delete. A variant with only `{value, done}` pre-created measured at main's level. A
    one-entry transition cache outside the image cut it to +0.8% but cost +334 B gzip of
    code, so it is not in; a cheaper delete transition is a follow-up.

## D2410: The harvester reads p2 session databases from a read-only copy
date: 2026-11-09 · by: slim · issue: 0149
p2 keeps sessions as SQLite databases (`~/.pi/p2/sessions/<id>.sqlite`, pi-durable's
schema; the pi messages are in the `entries` records' `model` field). extract.py reads
`*.sqlite` next to JSONL sessions, opened read-only (`mode=ro`), from a copy in
work/scratch (database plus `-wal`/`-shm`) so the live files are never opened by the tools.
Source ids of such runs start with `p2-`. Only `~/.pi/p2/sessions` is in scope: the rest of
`~/.pi/p2` holds credentials and keys and is never read. Outputs follow D021/D2281: counts
and names in reports, raw data in work/scratch only.
## D2430: The base image stores each flat string once
date: 2026-11-09 · by: strings · issue: 0150
`Heap::order_for_image` (the base build's last step, D2251) merges equal flat strings
(same kind, length and code units) into one cell and rewrites every reference to the
others.
- **Where the copies came from** (`djs heap` on a fresh REPL: 97 contents stored more
  than once, 2.9 KB): a builtin's `name` property, allocated per function (`"get
  [Symbol.species]"` seven times, `"map"` next to the atom `map`), constants in the
  templates' pools (`"enumerable"`, `"lastIndex"`) and the Error classes' `name` on both the
  constructor and the prototype, next to the atom table's copy.
- **Which copy stays:** the atom table's, else the first. Equal strings sort next to each
  other (content, then length, then atom copy first), so the kind grouping (D2251), the
  first-grown cells last (D2057) and the tail after `BASE_END` (D2063) keep their order.
  Strings in the tail and append buffers are left alone. The image shrinks by the copies;
  `OLD_END` and the GC threshold follow.
- **Why it is safe:** strings are immutable primitives compared by content, and nothing
  depends on two equal strings being different cells (no prefix string caches its hash,
  D2063).
- **Measured:** base 325,008 → 322,168 B, 5,852 → 5,750 cells; module 458,952 → 458,640 B
  gzip on its own.

## D2431: A two-level atom table: the base's atoms in the prefix, a VM's own in the tail
date: 2026-11-09 · by: strings · issue: 0150
**Why:** the base shrank the atom entries to their length (0043), and 1,460 of the 1,952
slots are holes left by D2253's dropped names. The array sat in the tail, so a session's
first new atom copied all of it into a cell 25% bigger (19.5 KB), and every session image
stored those 15–20 KB.
- **Layout:** `AtomTable` gains `BASE_ENTRIES`, `BASE_INDEX` (refs) and `BASE_COUNT`
  (`len(BASE_ENTRIES)`, read on every lookup) at 20, 24 and 28; the cell stays 32 bytes.
  `ENTRIES`/`INDEX`/`STRINGS` describe the atoms from `BASE_COUNT` on. A heap without a
  base has null base fields and a count of 0 and works as before. The verifier checks
  both levels.
- **Build:** `prepare_base_image` calls `Heap::split_atom_table(32)` after dropping the
  internal names: the entries and index become the base's and go to the builtin prefix
  (never written), and the tail gets an empty list with room for 32 atoms
  (`image::SESSION_ATOMS`) and a 64-bucket index.
- **Lookup:** `atom_entry` picks the level by index. Interning probes the VM's own index
  first and then the base's: a string is in one of them, the VM's index is small and its
  strings have cached hashes, while a miss among the base's strings hashes them again
  (D2063). One probe loop serves both levels.
- **Not done: compacting the holes.** Renumbering the base's atoms would take the holes
  out of the prefix (11.7 KB raw per VM, little in the coded module), but atom ids are
  stored untyped in many places: property tables, shape descriptors (sorted), template
  names and debug names, bytecode immediates, `global_decls` lists in the constant pools,
  and inline caches. Missing one would be a wrong-key bug. With the entries in the prefix
  they no longer cost session images, so it is a follow-up.
- **Measured:** base 322,168 → 322,720 B (+552, the room), the tail 27,160 → 8,016 B;
  a fresh REPL's raw session image 34,568 → 11,472 B, its container 1,177 → 1,096 B.
  An almost empty session changes the atom cells only where it adds (48 of 272 bytes of
  entries). Module 458,640 → 459,013 B gzip (+373: the base's coded image −335, code
  +~800 B raw: two levels in intern, `atom_str`, `atom_utf16`, `atom_entry`, verify),
  so 458,952 → 459,013 against main; the ceiling stays. vm_new 3.81 → 3.76 µs. Native
  instructions on the perf workloads: props +0.6%, push-pop +0.8%, json/jsonh +0.2%,
  json-date −4.6%, json-text −2.5%, the rest within ±0.3%; wasm within run-to-run noise
  (indexof ±1.5% between runs of the same module, json-date −2.9%, pi-like −1.5%). A
  dynamic `eq` in the probe saved ~270 B gzip but cost 60 instructions per intern (props
  +1.6%), so the probe stays generic.

## D2432: `djs heap` shows the session's own part by default
date: 2026-11-09 · by: strings · issue: 0150
With a reference to compare with, the `largest`, `strings`, `shapes` and `functions`
sections describe the cells that are new or changed against it; `--all` (or `--diff none`)
gives the whole image as before.
- Each starts with a line naming the view and one of the reference's totals
  (`unchanged from the reference: 809 flat strings, 25.4 KB`). JSON: `scope`
  (`session`/`all`) and `reference_part`.
- Retained sizes in the session view count only the session's cells a cell alone keeps
  alive. Duplicated strings: equal contents over the whole image; a session copy of a
  string the reference holds counts whole.
- The diff's differing words of a cell that changed size (a regrown table) compare over
  the words both cells have plus the nonzero new words, instead of the whole cell:
  `<atoms><entries>` had shown 19,536 of 19,536 bytes differing.
- The summary gives the session image's size (`session_image_bytes`, `base_end`).

## D2440: Prelude functions live in a realm table, not in captured Boxes
date: 2026-11-10 · by: bloat · issue: 0151
The prelude is one wrapper function whose top-level function declarations are the
builtins and their helpers. A builtin that called another one captured it like any
closure: the wrapper kept the callee in a Box, each caller had a capture descriptor and a
capture slot per callee in its function object. In the base that was 257 Boxes (4.1 KB),
652 capture descriptors (14.0 KB) and 11.0 KB of capture slots, for values that never
change after the prelude ran.
- **Compiler, privileged mode only:** a function declared at the top of a function that
  the Script itself contains (the prelude's wrapper, which runs once) and referred to from
  an inner function gets a slot of the realm's *prelude function table* instead of a Box
  (`Storage::Builtin`). References compile to `get_builtin idx` (in the wrapper too), the
  declaration's hoisting to `put_builtin idx, len` (the first store allocates the table).
  Assigning such a binding is an internal compiler error; bgen's subset has no such
  assignment. Functions declared deeper and all user code keep their closures.
- **Realm:** `Intrinsic::PreludeFunctions` holds the table (an internal ValueArray of 257
  entries, 2.1 KB, in the prefix). `get_builtin` is in `step`, `fast_step` and `run_hot`
  (three loads, like `get_cap`).
- **Registry:** the helpers are named by their source name, `%prelude:Name%`. The compiler
  gives the slot order (`compiler::prelude_function_names`); the registry compiles the
  prelude source into a scratch heap to get it. The 223 capture paths (`X~N`) are
  tombstones and their functions new entries: they were the fragile paths of D2371, and
  the new names stay as long as a helper keeps its source name. djs-cli's binary keeps the
  prelude source for this (+400 KB native, not in djs-wasm).
- **Measured** (base-families, on main after 0150): base 322,720 → 295,952 B (−26.8 KB,
  −8.3%), cells 5,752 → 4,848; module gzip 459,027 → 453,853 (−5.2 KB; the ceiling is
  lowered to 454,863);
  vm_new from base 3.77 → 3.42 µs. Bytecode is the same size (`get_builtin` is 3 bytes
  like `get_cap`). Wasm instructions (n=5, the outliers again at n=9) within ±0.8% on
  all 22 workloads; natively within ±0.7% except pushpop +1.8%, which never runs
  `get_builtin` (the same binary without the `run_hot` arm measures the same: layout).

## D2305: Keys iterators step over holes; `%IteratorFastDrain` takes keys and values runs in one step
date: 2026-11-09 · by: perf · issue: 0154
- **Holes:** D1800's fast step (`iter_plan`) declined on a hole for every Array Iterator
  kind. A keys iterator never reads the element (23.1.5.1), so it now produces `i` for every
  `i` below the length. `[...Array(n).keys()]` no longer takes the real `next` and a result
  object per element.
- **Runs:** the new leaf `%IteratorFastDrain(iterator, next, arr, n)` stores what the fast
  step would give, one CreateDataProperty per value into the Array `arr` from `n`, in one
  call: keys iterators to the end, values iterators up to the first hole, over Arrays and
  typed arrays. It stops where a store is not a plain dense one (`create_element_fast`),
  does nothing when `arr` is the iterated array (each step re-reads the length), and
  advances the iterator after the stores, so an allocation failure in between repeats the
  same stores. Fuel: one unit per four values, as the dense leaves charge. Used at the top
  of the loops of `%ArraySpread`, IterableToList, `%IteratorRest` and Array.from's
  iterable path without a mapping function.
- **Measured** (wasm, n = 1M, median of 5): `[...Array(n).keys()]` 593 → 44 ms, and
  `Array.from(Array(n).keys())` 642 → 45 ms, against quickjs-wasi's 55 and 54.

## D2306: Array.from(arrayLike, mapfn) runs through `%EachDense`
date: 2026-11-09 · by: perf · issue: 0154
Mode 8 of `%EachDense` is Array.from's mapping loop over an array-like. Its element k is
read live before each call. It is `o`'s dense element, or else the data property or
absence (undefined) found along a prototype chain of ordinary objects, Arrays and
functions. Anything else (an accessor, a proxy, another exotic object, an index past the
int atoms) hands the rest to the builtin's own loop. The callback gets two arguments
(`kValue, k`), and its result is stored into `a` at k as in map. The read sits outside the
per-element path of the other modes (`plain_get_index` out of line, after the dense
read): inlined, it cost forEach and higher-order 1–1.4%. `Array.from({length: 1M}, f)`:
168 → 101 ms, against quickjs-wasi's 80.

## D2307: Callback builtins on short arrays: no `%EachDense` for an empty array, and its call outside `step`
date: 2026-11-09 · by: perf · issue: 0154
- **The cost:** an exit from the tier-0 loop through `step_once`, `step` and
  `call_intrinsic` costs about 2.3k wasm instructions, roughly three plain calls. A `@fast`
  call's exit (`fast_leaf_call`) is cheap.
- **What changed:**
  - forEach, map, filter, some, every, find, findIndex and Array.from skip `%EachDense`
    when the length is 0;
  - `run_hot`'s `Decline` exit runs a `call_intrinsic` of `%EachDense` itself
    (`each_dense_decline`), with `step`'s fuel, `next` and fault handling, without the way
    through `step`.
- **Measured** (wasm instructions per call): forEach on [] 5.5k → 3.2k, map on [] 9.1k →
  6.8k, map on [x] 11.8k → 11.0k, filter on [] 10.6k → 7.1k, reduce on [x] 14.0k → 13.0k
  (quickjs-wasi: 1.6k, 5.0k, 6.6k, 5.0k, 2.5k).
- **Tried and dropped:**
  - answering `@fast` builtin intrinsics in `intrinsic_hit`: privileged code calls them as
    closures, never through `call_intrinsic`;
  - running `@fast` calls inside `call_closure` without leaving `run_hot`: every workload
    was 0.5–2% slower (code layout), and the exit it saves is cheap.
- **What remains:** pico's tree case (children arrays summed with map/reduce) went from
  700 to 665 ms, against quickjs-wasi's 310. About a fifth of it is the collector tracing
  the live tree (QuickJS counts references). The rest is the bytecode builtins' own
  instructions per call: frame, ToObject, LengthOfArrayLike, the species check and the
  callback frame. For reduce add the rest-parameter array and a second frame.
## D2510: No wasm JIT; ahead-of-time wasm for builtins stays `@native`
date: 2026-11-02 · by: jit · issue: 0153
djs does not build a JIT that emits wasm (docs/wasm-jit.md). A baseline template JIT of
djs bytecode fits the model: it exits only at instruction boundaries, checks fuel once per
block exactly, and writes back at every exit, so suspension, fuel, the moving collector
and images hold, with the interpreter as an exact oracle. The prototype measured 3.2–4.5x
over the interpreter on loops and calls once V8 has optimized the code (TurboFan), but
0.7–1.8x on Liftoff code, a break-even of about 0.2–0.5 ms of interpreted time per hot
function, and 0.3–2 ms of TurboFan compile per function. Harvested codemode scripts run
0.12 ms (median) in total, and the heavy workloads spend most of their time in leaf Rust
(pi-like: 12% in `run_hot`). Builtins keep their ahead-of-time path, `@native` (bgen's
Rust backend), justified per builtin by measurement (D1634). Revisit a baseline JIT only
for compute-heavy uses (> 10 ms of interpreted time in hot functions), and a bgen wasm
backend in a side module only if the module budget blocks measured `@native` wins.

## D2441: The hot loop's dispatch keeps no per-instruction values across calls
date: 2026-11-10 · by: bloat · issue: 0151
`run_hot` extracted the immediates (`w >> 8` as u16 and u32) before its `match` and kept
the opcode for `Exit::Decline` after heap calls. V8's TurboFan spills a value at its
definition when any path keeps it across a call, so every instruction stored two or three
values to the stack (TurboFan arm64 code of the dispatch, `--print-wasm-code`). Now the
arms extract their immediates themselves (`imm16!`/`imm32!`); the arms of several
opcodes read the opcode again after a heap call (`heap_call_op!`), single-opcode arms
name their opcode in `Exit::Decline`, and `iter_next` reads its offset again after its
step; the dispatch has no spill. Measured against main (wasm instructions, n=5): loop
−7.5%, tight −7.2%, prop1 −5.4%, call1 −4.9%, calls −4.9%, arrays −3.2%, forof −2.7%,
foreach −2.4%, props −2.4%, pushpop −1.9%, the others −1.0..+0.6%; natively loop −2.5%,
calls −2.2%. The module's gzip grows by 215 B (454,785 → 455,000), over the ceiling
(454,863): raised to 455,100 for this, to be lowered by 0151's L4 (whose smaller base
shrinks the module). The rule for later arms: a value computed for every instruction
must not live across a heap call in any arm. Check the dispatch block in V8's code after
changes to the loop (0151's work/scratch tf.sh: a named build, `node
--print-wasm-code-function-index`).

## D2442: GC growth counts the builtin prefix as a fixed allowance
date: 2026-11-10 · by: bloat · issue: 0151
`next_threshold` grew the arena by as much as the live data before the next collection,
and the live data included the builtin prefix (D2064), which is immutable and never
collected. Every byte cut from the base therefore made collections more frequent (0151:
−46 KB of base cost json +1.4% and pi-like +1.3% natively in collections alone). Now the
prefix counts as `gc::BASE_ALLOWANCE` whatever its size: the growth is the session's
live data (above `Roots::BASE_END`) plus the allowance, at least `MIN_GROWTH`. The
allowance is today's prefix, 288,160 B (the base image is 296,184 B with its per-VM
tail), so collections come exactly as before; it can be retuned on its own. Heaps
without a prefix keep the old rule; `restore_capacity`, which sees only an image length,
counts the whole image as live.
## D2308: Images hold no dead value-stack slots; embedder collections shrink a grown stack
date: 2026-11-09 · by: perf · issue: 0155
- **Dead slots:** the interpreter leaves popped values above `sp`. They are dead: it
  writes them before it reads them, and every collection clears them before it traces
  the stack. They are now zeroed:
  - in `Vm::image()`, the image writer for native hosts. `Vm::bytes()` stays the raw
    arena. The suspend round trips of the CLI, the REPL, difftest and the tests, and
    `session_bytes`, use `image()`;
  - in every embedder collection (`collect_heap`), which before set them to undefined.
    Zero is the Number +0, as a fresh stack has them, and the memset is smaller code;
  - in the old stack that growing leaves behind as garbage (`state::move_stack`).
    Without that, an uncollected image of equal histories differed across slicings in
    that garbage cell: a test of an uninterrupted run against one restored from
    `image()` every 997 fuel differed in 36 bytes.
- **No runtime cost:** the zeroing runs only when an image is taken, before a host
  collection and when the stack grows (a cold path).
- **wasm:** `vm_arena_ptr` is also the host's view for reading values, so it doesn't
  clear. The JS hosts take images only after `vm_collect`/`vm_seal` (Sandbox.bytes, pi's
  snapshot and prelude image), and those zero the dead slots.
- **Shrinking:** a stack grown for a deep recursion kept its capacity forever, and every
  later image carried it. Full and canonical embedder collections (not seals: the new
  stack would land after the cells a seal moves to the end, D2057) now move the live
  slots to a stack of max(256, 2·sp) slots when that is at most half the current one.
  The interpreter grows it again on demand.
  - Known gap: a stack that grew before a seal is old, and the first full collection
    after the seal frees it, shifting what follows once.
- **Measured** (session images, containers zstd against the base or prelude image):
  - pi session idle after `deep(2000)`: session image 459,976 → 199,880 B; container
    41,070 → 41,076 B.
  - pi session idle, plain: 200,024 B both ways; container 41,077 → 41,072 B.
  - REPL after `deep(300)`: raw image 597,600 → 566,880 B, session image 309,560 →
    278,840 B, container 37,329 → 37,405 B.
  - Containers barely change: zstd already squeezed the dead slots.
- **Other dead scratch:** none found. `djs heap` on a REPL snapshot and pi idle
  snapshots shows the stack as the only per-VM cell larger than its content; the job
  queue, the inbox and the rejections lists are small.
- **Size:** +91 B gzip.

## D2309: instr.py patches by checked anchors and splits user code from bytecode builtins
date: 2026-11-09 · by: perf · issue: 0155
scripts/perf/instr.py (the instrumented engine copy, 0081) broke silently twice: an
anchor that moved made a patch not apply, or apply in a dead place, so counters read zero
(the run_hot loop was never counted after 0107).
- **Anchors:** every patch is anchored on exact text that must occur a given number of
  times. Every mismatch is reported together, with the closest lines, and the script
  exits non-zero.
- **Counting:** the counters sit in one generated module (`prof.rs`) and are hooked at
  stable points:
  - the tier-0 loop's opcode decode and `step_once`;
  - frame pushes, native state arms, `call_intrinsic`;
  - `alloc_cell`, `after_collect`, and the entry of each collector.
- **Kind split:** a template's `PRIVILEGED` flag splits bytecodes into user code and
  bytecode builtins, per path (run_hot / step), per opcode and per function. Builtins are
  named by their registry paths (D2371).
- **Time split:** a sampling thread reads a "what runs now" word every 50 µs (user
  bytecode, bytecode builtins with the leaf intrinsics they call, `@native` builtins,
  collections).
- **Self-test:** `--self-test` builds, runs a script that must move every counter, and
  fails on any zero.

## D2443: Short opcode forms and shortened jumps, a recorded trade
date: 2026-11-10 · by: bloat · issue: 0151
The final-code emitter replaces the most frequent instructions by short forms when the
operand fits (`ops::short_form`): `get_loc0..3`, `put_loc0..3`, `set_loc0..3`,
`get_arg0..3`, `push0..3`, `call0..3` (operand in the opcode), one-byte forms of
`get_loc`/`put_loc`/`set_loc`/`get_arg`/`push_const`, `call_intrinsic8` (an intrinsic
below 256 with a fixed arity: the argument count is its declared parameters, which the
compiler already enforces), and `get_builtin_callee8` (`get_builtin; undefined`).
`CodeBuf::finish` shortens jumps whose offset fits a byte (`jump8`, `jump_if_*8`; start
short, widen until stable) and maps labels and line positions (`Code::map_pos`); the
re-emit drops code after a terminator up to the next label. Each short form behaves as its
wide form (`Op::wide`; same cost, flags and stack effect, checked at compile time). The
hot loop has one arm per short form, merged with the wide arm where that arm is large
(`call`/`call_intrinsic`: a second copy of `intrinsic_hit` lengthened V8's tier-up);
`exec` runs `call_intrinsic8`, `call0..3` and `jump_if_*8` itself, the rest in
`exec_short`. Phase-1 code and the compiler's passes see only wide forms.
- **Bought:** base 296,184 → 250,544 B (−45.6 KB per VM, −15%; bytecode 86.0 → 42.3
  KB, every builtin kept); a wasm instance after `vm_new` 19 → 18 pages; peak memory
  (mem-perturb n=6, MiB) pi-like 2.4 → 2.4, json 6.3 → 6.2, higher-order 15.0 → 14.9,
  arrays 13.8 → 13.7, strings 12.4 → 12.2, indexof and calls 1.2 → 1.1, strbuild 15.6 →
  15.8, json-date 3.8 → 4.3; pi bench warm start 0.43 → 0.41 ms, cold 5.04 → 4.99 ms,
  tool-heavy warm 3.06 → 2.87 ms, cold 7.30 → 7.22 ms.
- **Paid:** module gzip +377 B measured against main before 0155 (455,006 → 455,383),
  455,502 in the merge gate on main with 0155 (≈+420 B; ceiling raised to 455,600); wasm
  instructions against main with D2441 (n=5): loop −1.4%, tight −1.1%, forof −0.1%,
  higher-order +0.2%, json-replacer +0.5%, call1 +0.6%, arrays +0.7%, json-indent +0.7%,
  foreach +0.8%, strbuild +0.9%, push-pop +0.9%, jsonh +1.0%, calls +1.2%, props +1.2%,
  pi-like +1.3%, json-text/json/strings/prop1 +1.5%, indexof +2.7%, pushpop +3.9%
  (spread 1.6), json-date +4.8%. Natively run_hot is slower (loop +4.6%, calls +5.4%,
  prop1 +6.7%; not shipped).
- **Why the remainder is not steady state:** warm (repeated runs in one process) prop1
  and indexof are equal (2284 vs 2284, 400 vs 403 M per run); with `--no-liftoff` prop1
  is −2.9%. The once-only excess is V8's Liftoff phase before `run_hot` tiers up (a
  constant: prop1 at 1x/2x/4x iterations +62/+50/+45 M). json-date runs 4 collections
  instead of 3 in wasm: D2442 keeps the growth per cycle, but thresholds round to whole
  pages from offset 0, so a 46 KB smaller live size shifts the collection points, and a
  run with three or four collections can tip either way. Builds that differ only by a few
  `exec` arms moved single workloads by ±2% (V8 codegen of `run_hot`/`step_once`).
Accepted as a trade (coordinator): the per-VM saving and the faster pi bench against
first-run and GC-phase costs, with main 1–7% faster from D2441.

Three fixes found on the way, each worth having on its own:
- **+3 pages per wasm instance (djs-wasm):** a base below the page allocator's `HUGE`
  (256 KiB) was a dlmalloc block; the first VM's reserve moved the arena out of it and
  dlmalloc kept the hole. The base is now decoded (`lzrc::decode_with_capacity`, after the
  decoder's tables, so the buffer can grow in place) or injected into a page-backed
  buffer with the arena's capacity.
- **D2442 at load:** the base image stores its first threshold by the prefix rule, and
  `Heap::restore_capacity` takes the image's prefix (`djs_prelude::BASE_END` in wasm), so
  the first threshold is no longer past the arena's capacity.
- **`run_hot`'s fuel and end exits** are `Exit::General`, not `Decline(0)`: that payload
  made LLVM write the opcode's local at every loop head (two instructions per
  instruction in V8's baseline code).

## D025: The slim build's feature is `full`
date: 2026-11-03 · by: owner · issue: 0160
The default-on cargo feature that carries the families the slim build drops is named `full`
(cargo features must be additive); `unicode` is the second default-on feature. The build
without them is called slim (`djs-slim.wasm`).

## D026: Image compatibility keyed on what an image depends on
date: 2026-11-03 · by: owner · issue: 0164
Instead of refusing every image whose engine build hash (a hash of all djs-core sources)
differs, a snapshot is compatible when the things it actually depends on are unchanged:
the image/cell formats (layout hash), the opcode and intrinsic tables, the builtin prefix
(base hash), the snapshot container format, plus a hand-bumped compatibility version for
semantic changes those hashes can't see (docs/lessons-from-pico.md lesson 6).

## D2520: Arena access: one u64 compare by default, an optional unchecked `unsafe-heap` build
date: 2026-11-09 · by: audit · issue: 0161
Every arena field access goes through `heap/layout/access.rs`. The default checks
`off + N <= len` once, in `u64` so it cannot wrap on wasm32; LLVM then proves the slice
access in range and drops the two compares a `bytes.get(off..)` + `first_chunk` costs.
Module −5.2 KB gz (455.5 → 450.3), wasm instructions −0 to −4% on the perf workloads,
no `unsafe`. The cargo feature `unsafe-heap` (djs-core, passed through by djs-wasm; off by
default, not shipped) reads and writes without a check, with debug assertions; it is
the only `unsafe` in djs-core (lib.rs forbids it otherwise, and denies it outside that
module with the feature). It gives 443.1 KB gz and −8 to −15% instructions, but +2–3% on
call-heavy and tight loops (run_hot's register allocation). It is sound only for images
the engine produced: `Heap::verify` makes structure safe, but after it "heap code may still
panic on semantically odd data" (D1056), and those panics become undefined behavior inside
the instance. So a host may enable it only with trusted image storage. The feature is not
a build input of `SOURCE_HASH` (it changes no result or image), so the base built without
it loads into a build with it. The gate runs a verifier fuzz test and e2e with the
feature, and the difftest corpus in release with suspension and GC stress.
Measured and rejected for the safe build: masking with a power-of-two capacity (LLVM
cannot relate the mask to the slice length, so the check stays: larger code); reads that
return zero instead of panicking (larger: an else arm per site instead of one shared
panic); one check per 256-byte window shared by a cell's fields (merges in heap
functions, but the inliner spends the savings: −1.1 KB raw, +0.2 KB gz); `core::arch::wasm32::
unreachable` instead of the panic (+2.2 KB); `#[inline]` instead of `#[inline(always)]`
(no change). Per-cell views (`&[u8; N]` taken once, fields at constant offsets) remain the
way to close the rest of the gap, at call sites.
## D2550: Fuel stays per instruction; charging only at safepoints measured, not adopted
date: 2026-11-03 · by: speed · issue: 0162
pico charges fuel only at calls and taken backward jumps (docs/lessons-from-pico.md lesson
7). An experiment build (work/scratch/0162/fuel-safepoint.diff) did that in `run_hot`
only: no charge and no check on straight-line ops, the charge plus a `fuel <= 0` exit at
calls, returns and taken backward jumps; `step` unchanged. Against main after D2443, wasm
instructions (wmeasure.py, n=5): loop −6.8%, tight −6.7%, prop1 −6.1%, arrays −3.7%,
calls −3.7%, call1 −3.3%, props −3.1%, pushpop −3.0%, foreach −2.9%, tiny-reduce2 −2.4%,
forof −2.3%, push-loop −1.8%, higher-order −1.0%, tree-children −0.9%, push-pop −1.2%;
pi-like −0.2%; json, json-date, json-indent, json-replacer, json-text, jsonh, strings,
strbuild, indexof within ±0.5% (noise). So: at most ~7% on interpreter-bound loops and
nothing on codemode-like, JSON or builtin-heavy work. Adopting it would change G5 from
"stop between any two bytecode instructions" to "stop at a safepoint, at most one
straight-line block (bounded by the function's code size) later", change every fuel
count and every stress-mode stop point (suspend every 97), and need `step` and `run_hot`
to charge alike. Not worth that now; kept as an option for the owner.

## D2551: Large ASCII host strings are copied straight into a reserved heap string
date: 2026-11-03 · by: speed · issue: 0162
`vm_string_reserve(vm, len)` allocates a one-byte string of `len` zero units and returns
a handle to it; the host finds its data from `vm_arena_ptr`, the value's offset
(`vm_handle_bits`) and the layout's Str8 data offset, and fills it before anything
allocates. (Returning the address in the out buffer cost 40 more bytes gzip, past the
size ceiling; this costs +69.) `stringInPlace` (js/src/wire.ts) uses it from the
Sandbox's and pi engine's host values: for a string of at least 1024 units whose wire
encoding is STR_UTF8 with as many bytes as units (ASCII), it copies those bytes into the
reservation instead of staging the wire bytes for `vm_decode_wire`. The staging buffer no
longer grows to the largest result (it stays allocated for the instance's life), and the
transcode goes. Measured (Sandbox, async host function, a fresh instance per run): a 10
MiB result 21.4 → 11.4 MiB wasm memory and 3.2–5 → 2.1–2.9 ms; 1 MiB 3.4 → 2.4 MiB and
0.6–0.85 → 0.43–0.55 ms; 64 KiB 1.3 → 1.1 MiB; non-ASCII strings unchanged. Encoding
straight into the reservation with `encodeInto` (one copy fewer) was tried first: a
non-ASCII string then paid a reservation, a second encoding and a second arena growth (1
MiB with one em dash: 1.85 → 2.55 ms, 4.4 → 5.7 MiB); deciding from the wire encoding
costs it nothing. Two-byte strings and strings inside objects keep the wire path; pico's
result buffer with slices (lesson 3) is the larger step.
## D2411: The slim build: the `full` and `unicode` features, cut by a constant
date: 2026-11-03 · by: slim · issue: 0160
docs/slim-codemode.md's set as additive cargo features (D025): djs-core's default is
`["full", "unicode"]`; without them the build is slim (`scripts/build-wasm.sh --slim`,
target/djs-slim.wasm).
- **What `full` carries:** typed arrays, ArrayBuffer, DataView (incl. Float16Array and
  Uint8Array base64/hex), BigInt (global and literals), the Iterator helpers and the
  `Iterator` global, WeakRef and FinalizationRegistry, DisposableStack,
  AsyncDisposableStack, SuppressedError and `using`, decorators. `unicode` turns on
  `regexp-unicode-props` and `unicode-normalization`, which stay as finer features. The
  newest proposals and the Annex B leftovers of 0149's optional list stay in both builds.
- **Builtins:** a builtins/*.ts file with a `// @slim-omit` line, or a builtin tagged
  `@slim-omit` (SuppressedError, in error.ts), is left out of gen/prelude-slim.js, which bgen
  writes next to gen/prelude.js. Installs of kept builtins onto an omitted constructor
  (`%TypedArray%.prototype.toString`) go too. Calls from kept builtins into omitted ones stay
  (global reads behind `IsTypedArray` checks no slim value passes) and are listed in the
  slim prelude's header, so a new one shows in review. ToNumeric moved from bigint.ts to
  conversion.ts: the interpreter calls it in every build.
- **Rust:** `djs_core::FULL` (a `const` of `cfg!(feature = "full")`) guards what the slim
  build can't reach, so both builds type-check the same code, the slim one has no
  dead-code warnings, and the full one compiles to the same thing: the 40 leaf-intrinsic
  arms of the omitted families (`X if FULL =>`), the BIGINT tag in `Value::kind` and
  `as_bigint` (−7.5 KB gzip: the BigInt arithmetic behind every numeric operator goes),
  the wire format's bytes tag. The realm leaves the families' prototypes `undefined`
  (`SLIM_LEAVES_OUT`). Only the prelude source is chosen by `#[cfg]`. Guarding object
  classes in `ObjectClass::from_u32` the same way saved nothing (measured) and is not in.
- **Errors:** a missing global in `realm::ABSENT_GLOBALS` throws `ReferenceError: 'X' is
  not available in this build`; BigInt literals, decorators and `using` are a SyntaxError
  that says the same; the chain's last intrinsic arm says `%Name is not available in this
  build`.
- **Builds stay consistent:** djs-prelude, djs-wasm, djs-difftest, djs-test262 and djs-cli
  forward `full`/`unicode` (their default) and depend on djs-core without its defaults.
  djs-prelude's build script makes the base with a build-dependency djs-core whose
  features cargo resolves apart, so lib.rs asserts at compile time that the base's build
  hash is the library's. The umbrella `unicode` is left out of SOURCE_HASH (it changes
  nothing by itself); prelude-slim.js is in it.
- **Shapes (D2067):** no chain of `BUILTIN_RESULT_SHAPES` belongs only to the omitted
  families (Proxy, whose `{proxy, revoke}` is one, stays), so the list is the same in
  both builds.
- **Sizes:** full module 455,468 B gzip (main 455,502: −34 B); slim 390,849 B gzip
  (1.36x quickjs-wasi). Base image 250,544 B / 4,848 cells, slim 180,552 B / 3,495 cells
  (−28%, the prefix every VM copies).

## D2412: The registry lists what the slim build lacks
date: 2026-11-03 · by: slim · issue: 0160
tests/builtins/registry-slim.txt has the registry ids and paths that don't resolve in the
slim build (D2411), written by scripts/registry-update.sh from a slim build of the same tree
(target/slim) and checked by the slim build's registry test
(`Registry::check_build(heap, absent)`: exactly those must not resolve, all others must).
`djs_prelude::registry::absent`/`is_absent` give the set of the current build (empty in a
full one). A test checks `realm::ABSENT_GLOBALS` against the list's global names. A path
`X.prototype` whose global `X` the build lacks resolves through the realm's prototype slot,
so `Iterator.prototype` (every iterator's %IteratorPrototype%) keeps its id in the slim
build; exports name builtins with the same ids in both builds.

## D2413: Testing the slim build: its own legs in the gate
date: 2026-11-03 · by: slim · issue: 0160
The gate builds djs-slim.wasm beside the size build and tests the slim build natively in
target/slim:
- djs-core's `vm::slim_tests` (both builds: present in full, absent with the message in
  slim), `vm::frozen_tests` and `vm::registry`; djs-prelude's tests;
- difftest's `codemode/` and `codemode-harvest/` with `--suspend-every 97 --gc-stress
  --determinism` against the common ratchet;
- test262's active group: a runner whose djs-core lacks `full` (or `unicode`) puts
  tests/test262-slim.toml's `slim` (or `no-unicode`) skip group first (features, paths,
  and globals scanned for), and compares with tests/test262/expected-failures-slim.txt
  (BigInt literals in harness files and similar);
- pi's codemode tests on djs-slim.wasm with pi's prelude minus `lockdown()` on frozen
  builtins (`js/pi/pi-tests.ts --no-lockdown --expected tests/pi/slim-expected-failures.txt`:
  new failures fail; 19 expected, 11 of them also on the full module).

Its size has its own ratchet, `slim_ceiling_gzip` in tests/scores/size.tsv
(`budget.py --slim`), without the 1.5x budget it is under by design. The rest of the gate
runs the full build only.
## D2444: Code inline in the template, the rest of a user function in FuncInfo
date: 2026-11-03 · by: bloat · issue: 0151
A `FuncTemplate` holds its bytecode, its capture descriptors and its handler table inline,
in a `DATA` tail: `CODE_LEN` bytes of code at offset 48, then `CAPTURES` u32 descriptors,
then the 16-byte handler records, which fill the rest of the cell (the cell's padding is
shorter than a record, so `(size - 48 - CODE_LEN - 4 * CAPTURES) / 16` is the count and no
field is needed). The child templates and the debug info (script, source span, line table,
local names) move to a new `FuncInfo` cell (kind 21, `CHILDREN` an inline `Ref` tail),
referenced by `FuncTemplate::INFO`; a privileged template has none (no debug info, D1980,
and in the base no children), so a builtin's template is 48 bytes plus its code. This
replaces D1980's optional trailing fields: a tail and size-dependent optional fields
cannot share a cell, and the layout macro's `min` support is gone with them.

The interpreter's code register becomes `template + 48` and `code_len` a field of the
template head, one load fewer per call and return than the `CODE` reference to a `Bytes`
cell. `fclosure` reads its child through `INFO` (one load more, beside an allocation).
- **Bought:** base 250,544 → 231,960 B (−18.6 KB per VM, −7.4%; cells 4,848 → 3,916: the
  875 code cells and 57 handler `Bytes` cells are gone), every builtin kept; module gzip
  455,516 → 454,604 (−912 B). User functions get smaller too (one `FuncInfo` replaces the
  32 optional bytes, the code cell, the children `RefArray` and the descriptor array).
- **Neutral:** wasm instructions against main (n=3, all 22 workloads) −2.7%..+1%, except
  pushpop +2.9% and indexof +1.0%, both noise on re-measure (n=7: −1.9% and +0.0%; warm
  per-run 3878 vs 3896 and 400 vs 400 M). Peak memory (mem-perturb n=6) equal, strbuild
  15.8 → 15.4 and json-date 4.3 → 3.7–4.3 MiB. A wasm instance after `vm_new` stays at 18
  pages.

## D2310: A twin stays only where it pays its bytes
date: 2026-11-09 · by: perf · issue: 0159
`@native` state machines and `@fast` entries (D1632, D1850) are second implementations
of builtins that bytecode already provides. The bloat audit put them at 10.6 + 8.3 KB gz.
Each was measured alone, turned off at run time in a measurement build (a byte mask in
the module, so every variant had identical code and layout):
- **Benchmarks:** wasm instructions, node's default tiering (Liftoff, then TurboFan),
  on the 22 workloads, pi-like, an async microbenchmark and the pi bench (djs only).
  - The relative numbers are net of an empty run.
  - Noise is about ±0.5% on workloads and about ±1% on the pi bench.
  - A first pass with `--no-liftoff` hid real costs: dropping `indexOf`'s entry cost
    `strings` +5% under default tiering and nothing under TurboFan alone.
- **Calls:** each twin's calls, counted in the workloads, the pi bench and the 310
  codemode corpus cases.
  - The corpus scripts are short: their calls of any twin are at most a few thousand.
    The native run-to-run noise of the whole corpus (±4%) is larger than any twin's
    effect there, so the corpus decided nothing.

**The rule.** A twin stays if turning it off costs:
- (a) at least 1% on pi-like or on the pi bench (the codemode-shaped work; a pi-bench
  effect counts only if the twin has calls there), or
- (b) on some workload, at least 10% per KB gz of the twin's code (estimated as its
  functions' raw bytes × 0.385, the module's code ratio).

Twins that call each other are judged together: the sort pair (`SortList`,
`ArrayPrototypeSort`) and the five `JSON.stringify` serializer natives. Those two groups
were kept at the coordinator's call: sort with a comparator +13.5% on higher-order, and
`JSON.stringify` with a replacer +14% as a group. Under (b) alone they fall short (2.4 and
4.2 KB gz).

**The set as a whole:** with an earlier drop set off (the rule's set plus `indexOf`,
`Array.prototype.includes` and the sort native, whose costs only default tiering
showed), pi-like was +1.5% and the pi bench +1.7% (pi-like +0.9% in another run:
noise). Three cheap entries with calls
in both, `ToPropertyKey`, `ToString` and `Array.prototype.join` (about 250 B gz),
were kept back, which brought the two to +0.40% and +0.32%. The full set would have
saved 198 B gz more.

**Result:**
- **Kept:** 18 `@fast` entries and 8 `@native` builtins. **Dropped:** 54 entries and
  the natives `GetPrototypeFromConstructor` and `PromiseReactionJob`.
  - Both of those natives were as fast or faster as bytecode: pi-like −0.7% without
    the first; the async microbenchmark within ±1% without the second.
- **Code:** `fast.rs` loses its `Math` entry kind.
- **Size:** module 455,081 → 448,823 B gz (−6,258) on the branch. On main after
  the slim build (0160): full 450,369 → 445,107 (−5,262), slim 385,842 → 379,276
  (−6,566). Both ceilings lowered (446,131 and 380,300).
- **Speed, real build against main** (wasm instructions, n = 3):
  - all 22 workloads between −2.2% and +0.92%; pi-like +0.45%;
  - the pi bench +0.1% (within noise); async +0.65% (n = 5).

Per twin (alone off; workloads where it has calls; `–`: no calls in any workload):

| twin | kind | raw B | est. gz B | pi-like | pi bench | worst workload | corpus calls | kept |
| --- | --- | ---: | ---: | ---: | ---: | --- | ---: | --- |
| ToString | @fast | 134 | 52 | +0.43% | -0.26% | json-text +1.8% | 330 | yes |
| ToNumber | @fast | 233 | 90 | +0.00% | +0.59% | json-date +0.1% | 22 | no |
| ToIntegerOrInfinity | @fast | 233 | 90 | +0.00% | +0.00% | – +0.0% | 0 | no |
| ToLength | @fast | 281 | 108 | -0.10% | +0.01% | pi-like -0.1% | 1 | no |
| LengthOfArrayLike | @fast | 200 | 77 | +1.18% | +1.71% | foreach +2.3% | 2190 | yes |
| ToPropertyKey | @fast | 215 | 83 | +0.00% | +0.94% | json-indent +0.3% | 2273 | yes |
| ThisToString | @fast | 169 | 65 | +0.00% | +0.30% | – +0.0% | 0 | no |
| RequireThisCoercible | @fast | 72 | 28 | +0.00% | -0.31% | – +0.0% | 31 | no |
| IsRegExp | @fast | 65 | 25 | +0.00% | +0.00% | – +0.0% | 0 | no |
| ClampPosition | @fast | 341 | 131 | +0.00% | +0.00% | – +0.0% | 0 | no |
| RelativeIndex | @fast | 374 | 144 | +0.00% | -0.02% | – +0.0% | 0 | no |
| StrLen | @fast | 124 | 48 | +0.00% | +0.08% | – +0.0% | 28 | no |
| StringConstructor | @fast | 119 | 46 | -0.28% | -0.13% | pi-like -0.3% | 523 | no |
| StringPrototypeIncludes | @fast | 401 | 154 | +35.00% | +5.11% | pi-like +35.0% | 86 | yes |
| StringPrototypeIndexOf | @fast | 461 | 177 | +0.00% | +0.31% | strings +5.1% | 45 | yes |
| StringPrototypeLastIndexOf | @fast | 559 | 215 | +0.00% | +0.00% | – +0.0% | 0 | no |
| StringPrototypeStartsWith | @fast | 427 | 164 | +0.00% | +0.49% | – +0.0% | 597 | no |
| StringPrototypeEndsWith | @fast | 429 | 165 | +0.00% | -0.19% | – +0.0% | 38 | no |
| StringPrototypeSlice | @fast | 414 | 159 | +0.00% | +1.00% | push-pop +4.6% | 190 | yes |
| StringPrototypeSubstring | @fast | 567 | 218 | +0.00% | +0.00% | – +0.0% | 0 | no |
| StringPrototypeCharAt | @fast | 306 | 118 | +0.00% | +1.08% | push-pop +4.1% | 0 | yes |
| StringPrototypeCharCodeAt | @fast | 352 | 136 | +0.00% | +0.77% | push-pop +9.3% | 0 | yes |
| StringPrototypeCodePointAt | @fast | 420 | 162 | +0.00% | +0.00% | – +0.0% | 0 | no |
| StringPrototypeAt | @fast | 381 | 147 | +0.00% | +0.00% | – +0.0% | 0 | no |
| StringPrototypeRepeat | @fast | 511 | 197 | +0.00% | +0.03% | push-pop -0.7% | 0 | no |
| StringPrototypePadEnd | @fast | 18 | 7 | +0.59% | +0.22% | pi-like +0.6% | 0 | no |
| StringPrototypePadStart | @fast | 18 | 7 | +0.84% | +0.07% | pi-like +0.8% | 3 | no |
| StringPrototypeToLowerCase | @fast | 189 | 73 | +0.00% | +0.02% | – +0.0% | 23 | no |
| StringPrototypeToUpperCase | @fast | 189 | 73 | +0.00% | +0.00% | – +0.0% | 0 | no |
| StringPrototypeToLocaleLowerCase | @fast | 189 | 73 | +0.00% | +0.00% | – +0.0% | 0 | no |
| StringPrototypeToLocaleUpperCase | @fast | 189 | 73 | +0.00% | +0.00% | – +0.0% | 0 | no |
| StringPrototypeTrim | @fast | 14 | 5 | +0.00% | -0.02% | – +0.0% | 202 | no |
| StringPrototypeTrimStart | @fast | 14 | 5 | +0.00% | +0.00% | – +0.0% | 0 | no |
| StringPrototypeTrimEnd | @fast | 14 | 5 | +0.00% | +0.23% | – +0.0% | 3 | no |
| StringPrototypeSplit | @fast | 2139 | 824 | +30.87% | +4.37% | pi-like +30.9% | 172 | yes |
| StringPrototypeReplace | @fast | 475 | 183 | +0.00% | +0.16% | – +0.0% | 7 | no |
| StringPrototypeReplaceAll | @fast | 778 | 300 | +0.00% | +0.00% | – +0.0% | 0 | no |
| NumberConstructor | @fast | 236 | 91 | +1.23% | -0.04% | pi-like +1.2% | 4 | yes |
| GlobalParseInt | @fast | 370 | 142 | +0.00% | +0.23% | – +0.0% | 2 | no |
| GlobalParseFloat | @fast | 268 | 103 | +0.00% | +0.00% | – +0.0% | 0 | no |
| GlobalIsNaN | @fast | 127 | 49 | +0.00% | +0.00% | – +0.0% | 0 | no |
| GlobalIsFinite | @fast | 134 | 52 | +0.00% | +1.13% | – +0.0% | 12 | no |
| NumberPrototypeToString | @fast | 437 | 168 | +0.00% | +0.00% | – +0.0% | 0 | no |
| NumberPrototypeToFixed | @fast | 382 | 147 | +0.00% | -0.01% | – +0.0% | 39 | no |
| MathMax | @fast | 359 | 138 | +0.00% | +0.26% | – +0.0% | 61 | no |
| MathMin | @fast | 359 | 138 | +0.00% | +0.56% | – +0.0% | 12 | no |
| MathAbs | @fast | 0 | 0 | +0.00% | +0.00% | – +0.0% | 0 | no |
| MathCeil | @fast | 0 | 0 | +0.00% | +0.00% | – +0.0% | 0 | no |
| MathFloor | @fast | 0 | 0 | +0.00% | +0.00% | – +0.0% | 0 | no |
| MathRound | @fast | 0 | 0 | +0.00% | -0.04% | – +0.0% | 5 | no |
| MathSign | @fast | 0 | 0 | +0.00% | +0.00% | – +0.0% | 0 | no |
| MathSqrt | @fast | 0 | 0 | +0.00% | +0.00% | – +0.0% | 0 | no |
| MathTrunc | @fast | 0 | 0 | +0.00% | +0.00% | – +0.0% | 0 | no |
| MathExp | @fast | 0 | 0 | +0.00% | +0.00% | – +0.0% | 0 | no |
| MathLog | @fast | 0 | 0 | +0.00% | +0.00% | – +0.0% | 0 | no |
| MathLog2 | @fast | 0 | 0 | +0.00% | +0.00% | – +0.0% | 0 | no |
| MathLog10 | @fast | 0 | 0 | +0.00% | +0.00% | – +0.0% | 0 | no |
| MathSin | @fast | 0 | 0 | +0.00% | +0.00% | – +0.0% | 0 | no |
| MathCos | @fast | 0 | 0 | +0.00% | +0.00% | – +0.0% | 0 | no |
| ObjectPrototypeHasOwnProperty | @fast | 46 | 18 | +0.00% | +0.00% | – +0.0% | 0 | no |
| ObjectHasOwn | @fast | 71 | 27 | +0.00% | +0.00% | – +0.0% | 0 | no |
| ArrayIsArray | @fast | 130 | 50 | +0.00% | +0.00% | – +0.0% | 0 | no |
| ArraySpeciesCreate | @fast | 444 | 171 | +5.10% | +0.19% | pi-like +5.1% | 422 | yes |
| ArrayPrototypePush | @fast | 187 | 72 | +0.47% | +1.84% | pushpop +46.7% | 102 | yes |
| ArrayPrototypePop | @fast | 88 | 34 | +0.00% | +1.07% | pushpop +56.8% | 0 | yes |
| ArrayPrototypeIndexOf | @fast | 484 | 186 | +0.00% | +0.03% | indexof +22.3% | 0 | yes |
| ArrayPrototypeLastIndexOf | @fast | 505 | 194 | +0.00% | +0.00% | – +0.0% | 0 | no |
| ArrayPrototypeIncludes | @fast | 401 | 154 | +0.00% | +0.45% | foreach +2.5% | 86 | yes |
| ArrayPrototypeSlice | @fast | 829 | 319 | +4.29% | -0.01% | arrays +5.1% | 18 | yes |
| ArrayPrototypeJoin | @fast | 303 | 117 | +0.59% | +0.02% | json-text +1.8% | 272 | yes |
| ArrayPrototypeFill | @fast | 421 | 162 | +0.00% | +0.00% | – +0.0% | 0 | no |
| StringFromCharCode | @fast | 557 | 214 | +0.00% | +0.22% | strings +38.8% | 0 | yes |
| ArrayPrototypePush | @native | 1496 | 576 | +0.52% | +0.61% | push-pop +72.0% | 3 | yes |
| SortList | @native | 6105 | 2350 | +0.43% | -0.25% | higher-order +13.5% | 11 | yes |
| ArrayPrototypeSort | @native | 1816 | 699 | +0.59% | -0.12% | higher-order +2.3% | 11 | yes |
| GetPrototypeFromConstructor | @native | 824 | 317 | -0.74% | -0.26% | json-date -0.2% | 385 | no |
| SerializeJSONProperty | @native | 4967 | 1912 | +0.00% | -0.07% | json-replacer +6.5% | 28 | yes |
| JSONStackIncludes | @native | 609 | 234 | +0.00% | +0.02% | json-replacer -0.1% | 13 | yes |
| JSONJoin | @native | 1627 | 626 | +0.00% | +0.21% | json-replacer +2.8% | 13 | yes |
| SerializeJSONObject | @native | 2363 | 910 | +0.00% | -0.04% | json-replacer +3.0% | 11 | yes |
| SerializeJSONArray | @native | 1401 | 539 | +0.00% | -0.00% | json-replacer +1.4% | 2 | yes |
| PromiseReactionJob | @native | 2495 | 961 | +0.00% | -0.18% | – +0.0% | 2187 | no |
## D2265: Images carry a dependency descriptor; loading and containers check it, not the build hash
date: 2026-11-03 · by: snapshot · issue: 0164
D026 made concrete. The roots cell holds six u32 (`vm::ImageDeps`): the layout hash
(`Roots::VERSION`), the feature set (`FEATURES`, D2267), the hand-bumped
`vm::COMPAT_VERSION` (`COMPAT`, at the old `BUILD` offset 48), `OPS_HASH` (`OPS`),
`INTRINSICS_HASH` (`INTRINSICS`) and `RECORDS_HASH` (`RECORDS`, D2266); `Vm::new` writes
them. `Vm::check_image` compares them in that order (the layout first: the other fields
are read at its offsets) and refuses with `LoadError::Incompatible { dep, found,
expected }` (replaces `BuildMismatch`), naming the component: "the image is from a djs
with another opcode table (image: X; this build: Y)". The snapshot container is format 3
(the header changes, D024): the six words at 8..32 and the FNV-1a 64 of the base image's
builtin prefix at 32, reference fields from 40, header 68 bytes; js/src/snapshot.ts and
djs-cli's container compare them with the module's base image (its roots and prefix hash)
before decompressing, and the message names the component ("another base image" for the
prefix; features by name). Session images are format 2: offset 8 holds `ImageDeps::hash`
of the base instead of the build hash. `vm::BUILD_HASH` stays as the engine's identity
(`djs_build_hash`, Sandbox journals, pi's fingerprint): journals replay step by step and
need the exact build; images only need the numbering. The descriptor guards numbering,
not meaning: semantic changes need a `COMPAT_VERSION` bump (D2268). Measured on main's
history (`churn.py --deps`, docs/snapshot-versioning.md "Since D026"): a raw image
survives 51% of merges instead of 30%; a container, which also needs the same base image,
47% instead of 31% (the 129 merges whose base images were all built), 60% instead of 47%
over the last 30.

## D2266: RECORDS_HASH: the id tables images store numbers of, read from the sources
date: 2026-11-03 · by: snapshot · issue: 0164
djs-core's build script hashes the regions `crates/djs-core/compat_inputs.rs` lists (the
same file is `include!`d by `vm::compat_tests`): the items `ObjectClass`, `impl PropFlags`,
`cells::flags`, typed array `Kind`, `Continuation` and its impl, realm `Intrinsic`,
`RunState`, `func_flags`, `capture_desc`, `global_decls`, value `tag`, the `FAST` table,
the whole generated `natives.rs` (a suspended native frame stores its state machine's
state), and every top-level `const` of src/heap/, src/interp/, src/builtins/, src/regexp/,
value.rs and bytecode/{template,handlers,lines}.rs, except intrinsic ids by name
(`= id("Name")`, which `INTRINSICS_HASH` covers). Text with comments and whitespace removed
(string and char literals kept), so doc edits and formatting keep the hash and a changed
number, a new entry or a renamed constant change it; a region that is not found fails the
build. Reading sources rather than meanings is conservative: some changes refuse images
that would still load (a tuning constant), and what a slot means in code is the
compatibility version's job. Alternative: hash only hand-listed tables (misses the record
slot constants, which change with every new internal record).

## D2267: Cargo features are classified as image features or image-neutral
date: 2026-11-03 · by: snapshot · issue: 0164
`compat_inputs.rs` `IMAGE_FEATURES` (`full`, `regexp-unicode-props`,
`unicode-normalization`; append only) are bits of `vm::FEATURES` (`Roots::FEATURES`):
a slim image in a full build is refused as "another feature set (image: full, ...; this
build: ...)". `IMAGE_NEUTRAL_FEATURES` (`default`, `unicode`: umbrellas; `unsafe-heap`:
heap access, not contents) change no image. A feature in neither list fails djs-core's
build, so each new one is classified. The JS host mirrors the names (`FEATURE_NAMES`,
checked against the file by snapshot.test.ts).

## D2268: When to bump COMPAT_VERSION
date: 2026-11-03 · by: snapshot · issue: 0164
`vm::COMPAT_VERSION` (1) is bumped by hand, in the change that needs it, when an image
written before the change would load (every hash equal) but be read differently after it.
The guideline is TESTING.md "Snapshot compatibility"; AGENTS.md points to it. A reviewer
asks for it like for a test: a forgotten bump lets an old image restore into an engine that
misreads it, which the verifier only catches when the structure breaks.
## D2521: Vec pushes out of line outside the hot loops (`PushOut::push_out`)
date: 2026-11-09 · by: audit · issue: 0161
`Vec::push` inlines a capacity check and a call to `RawVec::grow_one` that takes the
vector by reference, so the vector is spilled to the shadow stack at every site; at ~370
sites that was ~13 KB of wasm. `crate::PushOut::push_out` (djs-core/src/buf.rs) is
`push` in one `#[inline(never)]` function per element type (and for `String` and
`char`). 343 sites in 54 files use it now: everything except JSON (`json/`), the dense
array builtins, the bytecode emitter and lexer, the resolver and peephole passes, the
stack walker, and the files 0151 is editing (heap cells/walk/atom/compact, vm image/mod,
the template, run.rs and hot.rs), which may follow in a separate merge. Measured: module
−3.2 KB gz (450.3 → 447.0); wasm instructions within ±1.3% on the 15 perf workloads, all
but one lower; compile instructions −0.9 to +0.6% (the noise of wcompile is ~1%). All
pushes out of line would be −4.8 KB gz but +1.0–1.7% on json, json-text and pushpop and
+1% compile. `push_out` takes values of at most 16 words (a compile-time assertion): a large value
passed to a call is built in the caller's frame first, which grew `parse_arrow`'s debug
frame from 64 to 864 bytes and overflowed the stack budget of js/test/nesting.test.ts
(D1941); an inline `push` builds it in place. New code uses `push_out` unless the push is
in a measured hot loop or the value is large;
`scripts/size/vec-sites.py --check scripts/size/vec-sites.txt` reports inlined pushes,
drops, `with_capacity`, `reserve` and allocation-error paths per function against the
baseline.

## D2445: A closure's length and name come from its template until changed
date: 2026-11-03 · by: bloat · issue: 0151
A closure (`ObjectClass::Function`) has two header flags, `OBJ_LAZY_LENGTH` and
`OBJ_LAZY_NAME`: while set, the object has an own `length` (the template's `LENGTH`) and
`name` (the string of the template's `NAME` atom, empty for none), both `{writable:
false, enumerable: false, configurable: true}`, that are not stored. `find_own` reports
them (`Loc::LazyLength`/`LazyName`), `[[OwnPropertyKeys]]` lists them before the stored
string keys, `[[Set]]` fails on them like on any read-only property, and `[[Delete]]`
clears the flag (the other keys keep their order). Any other change that validation
accepts (a redefinition with a new value or attributes, a freeze) first stores both as
ordinary properties at the front (`materialize_lazy`: the object becomes a dictionary
whose entries move up; rare, and it allocates before it commits); a redefinition that
changes nothing changes nothing.
- User closures: `function_properties` sets both flags when the template's name is a
  string atom or none (an integer key's name, `{1() {}}`, is still stored). A closure
  then needs no property definition; ordinary functions still get their `prototype`.
- Builtins: at base-image build (`image::lazy_builtin_props`, before the prelude's shape
  tree is replaced), a builtin closure whose first two stored properties are the
  `length` and `name` `%Install*` defined, with a name that is already a string atom and
  a template no other closure shares, moves them into its template (`LENGTH`, `NAME`,
  `func_flags::LAZY_NAME`, which keeps the name when `drop_internal_names` clears the
  others). Names that are not atoms ("get size", "[Symbol.iterator]", 59 of them) stay
  stored: interning them grew the atom index by 4 KB.
- Inline caches: a shape does not show the two keys, so `exotic_own` makes a function's
  `length` and `name` look up rather than trust a shape compare for prototype reads and
  additions, and `shaped_add` refuses a lazy object.
- `BUILTIN_RESULT_SHAPES` precreates `prototype` alone (writable for functions, not for
  classes) and `length, name` for builtin and bound closures, instead of the
  `length, name, prototype` chains.
- **Bought:** base 231,960 → 217,608 B (−14.4 KB per VM; property storage 37,088 →
  22,528); a wasm instance after `vm_new` 18 → 17 pages; tree-children (pico's tree
  benchmark: a closure per call) −11.9% wasm instructions cold, −14% warm; every closure
  32 B smaller (no values block for `length` and `name`). Peak memory (mem-perturb n=6)
  −0.1 MiB on most workloads.
- **Paid:** module gzip +652 B (444,537 → 445,189; the materialization and lookup
  paths). Other workloads within noise (n=3 cold −1.3..+2.3%, re-measured n=7 −1.1..+1.3%;
  warm ±1%, pi-like −0.7%).
## D2540: A 128 KiB wasm stack; parser nesting limit 550
date: 2026-11-09 · by: heap · issue: 0158
Supersedes D1941's size. djs-wasm links with `-zstack-size=131072` (two pages of every
instance instead of four; `DJS_WASM_STACK` still overrides it), and the parser's
`MAX_DEPTH` goes from 1000 to 550 units: about 180 nested parentheses, arrays, objects or
template substitutions, 270 calls or arrows, 545 blocks, statements or `try`s, 60 classes
(qjs accepts ~920 parentheses). 650 was measured first, with 10–12% to spare; after
rebasing onto 0161 (out-of-line `Vec` pushes) blocks and `try`s trapped at 612 and 611
in the debug build (were 724), so 550 (blocks 545 of 611, classes 60 of 77, functions
109 of 148: at least 10% spare; the release module has more). Measured on the debug wasm (the gate's nesting test): at
1000 units, the deepest nesting that did not trap at 128 KiB was 0.72 of the accepted
depth for blocks and `try`s, 0.73 for classes; at 650 every construct reaches its
SyntaxError with 10–12% of the stack to spare (classes 71 accepted, trap at 82; blocks
645, trap at 724). Runtime recursion does not use the Rust stack in depth: JS calls,
JSON parse/stringify, `join`, proxy/bound/prototype chains, getters, a 300k-deep list
through the GC, regexps, sort, async recursion, `toString`, `flat` give identical results
at 256 and 128 KiB. test262 `language/` and the native depth test pass unchanged.

The regexp parser's own nesting limit goes from 255 to 201 (200 nested groups, which
test262's S15.10.2.8_A3_T15 needs): in the release module (opt-level s inlines more into
the recursion than the debug build), 243 nested capture groups and 236 lookaheads
trapped at 128 KiB; 200 leave ~15%. Non-capturing groups and `v`-mode classes took 254
levels. Stack overflow at run time stays a RangeError (D013): the interpreter's call
depth is in the arena, not on the Rust stack.

## D2541: Instance memory: the arena kept at memory's end, lean start capacity, the coded base outside memory
date: 2026-11-09 · by: heap · issue: 0158
A fresh Sandbox is 11 wasm pages (704 KiB) instead of 18: 2 stack (D2540), 3 static
data, 2 dlmalloc, 4 base. Three changes:
- **The page allocator keeps the arena at the end of memory** (djs-wasm allocator.rs,
  extends D2053). Whatever lands right after the arena blocks its growth in place; a
  page log over all workloads showed every growing one moved its arena once, at its
  initial 10 pages, because dlmalloc's first segment (or the 2-page segment a
  collection's side tables need) came right after it, and dlmalloc never gives segments
  back. Now:
  - `PageAlloc::prime` gives dlmalloc a 2-page first segment before anything else (one
    page left the pi cold start and json/jsonh 0.4–0.8 MiB worse);
  - blocks of 16 KiB up to `HUGE` go to dlmalloc only while its segments have room
    (`Sys::no_grow`), else they are page runs of their own, tracked in a 16-slot list
    and freed back to the page pool; without this arrays, higher-order and strbuild ended
    3–4 MiB higher;
  - the largest huge block is the *top* block; a buffer that a realloc turns into a page
    run right behind it starts its own size further on (`Keep`), so that the top block
    can still take the buffer's copy in place (json 6.7 → 5.8, jsonh 6.3 → 5.4 MiB).
- **Lean start capacity.** A VM from the base, or restored from an image
  (`djs_image_staging`), reserves the image plus one page (`Heap::image_capacity`)
  instead of its first GC threshold plus slack (D1942, D1990), which was 2x the base and
  never touched by most sandboxes. Thresholds and collection points are unchanged. A host
  that passes a size hint (`vm_new_reserved`, `Vm::from_base_reserved`; pi's cold start)
  still gets `restore_capacity` (`Heap::start_capacity` chooses).
- **The coded base is the custom section `djs_base`** (djs-prelude build.rs writes it as
  a `pub` static array with `#[link_section]`; with `#[used]` or through
  `include_bytes!` the bytes stayed in the data segment too). Hosts hand it to an
  instance with the new export `djs_base_coded_ptr(len)` when they have no decoded base
  to inject (`provideCodedBase` in js/src/snapshot.ts; `provideBase` and `baseReference`
  call it). The instance drops it once it keeps a decoded copy. Without it there is no
  base: `djs_base_ptr` returns 0 and `vm_new` fails with a message. ABI version 2.
  −1 page and ~40–50 KiB resident per instance (the data segment is copied into memory
  at instantiation).

Measured against main (mem-perturb, 24 runs each, mean MiB; every distribution is
min = max within 0.1 except json-replacer):

| workload | main | now |
| --- | ---: | ---: |
| empty, call1, calls, loop, tight, indexof, prop1, pushpop | 1.10 | 0.70 |
| pi-like | 2.40 | 1.60 |
| push-pop | 2.00 | 1.19 |
| foreach, forof | 4.70 | 3.80 |
| json-indent | 5.40 | 4.70 |
| json-replacer | 5.34 | 4.66 |
| json | 6.20 | 5.80 |
| jsonh | 5.80 | 5.40 |
| props, json-text | 1.30 | 1.20 |
| arrays | 13.70 | 13.10 |
| higher-order | 14.90 | 14.30 |
| strbuild | 15.80 | 15.30 |
| strings | 12.20 | 12.00 |
| **json-date** | **4.30** | **5.20** |

pi bench, wasm pages per instance: warm start (`return 1`) 23 → 14, warm tool-heavy
25 → 24, cold (prelude run each time) 26 → 24; the process's first instance 36 → 30.
Resident per sandbox ~650 → ~610 KiB (noisy). Module 450,277 → 451,262 B gzip (+985,
the allocator and the base hand-over). Wasm instructions within ±1%: a first run with n=5 under load
showed json-text +4.4%, json-date +3.9%, indexof +2.6% (spreads 2–4%); re-measured on a
quiet machine, n=9, main before (d09c8c39) and after (47d32099): json-text +0.9%,
json-date −0.6%, indexof +0.9%, json and loop ±0.0% (spreads ≤0.9%). Noise, not part
of the trade.

**Accepted trade (coordinator): json-date +0.9 MiB.** JSON.stringify builds a large
result in a Rust `Vec`, which grows into a page run right behind the arena; copying it
into a heap string grows the arena (to its threshold, +9 pages for a 4-page buffer)
while the buffer is alive, so the arena moves once. main hid this in the 10-page hole
its first arena move left. The real fix is in JSON.stringify: not holding a large Rust
buffer while the arena grows (write large results straight into a heap string).

Measured and rejected (single runs of an instrumented build, final wasm pages, against
the taken variant's):
- a gap of half or a quarter of the top block behind growing buffers: json-date 85 →
  74/64 pages, but unused gaps at the end of runs: json 95 → 134/114, jsonh 88 →
  124/106, json-replacer 76 → 108/92, strings 194 → 219/203, foreach 63 → 89/76;
- moving the arena on purpose once it outgrows 8–12 pages, to leave main's hole for
  transients: growing runs back at main's numbers (only small VMs gain), and
  json-text/props 20 → 28 at 10 pages;
- arena growth by eighths instead of doubling up to the threshold: no gain, json worse;
- letting dlmalloc release segments, and trimming it when the arena is blocked: no
  change (the blocking segment still holds live blocks after a compile);
- a dlmalloc prime of 3 or 4 pages: small VMs +1–2 pages, nothing else better;
- no medium page runs: arrays 209 → 258, higher-order 230 → 277, strbuild 245 → 287,
  json-indent 76 → 116 pages.
## D2522: `Vec::with_capacity` out of line (`buf::vec_with_capacity`)
date: 2026-11-09 · by: audit · issue: 0161
Like D2521's pushes: `Vec::with_capacity` inlines the allocation call, the vector's
spill and the allocation-error path at every site. `crate::buf::vec_with_capacity` is one
`#[inline(never)]` function per element type; 60 sites use it (all but run.rs and the
template, which 0151 is editing). Module −1.3 KB gz (442.1 → 440.8), the 15 perf
workloads within −2.1 to +0.5% (an allocation is a call anyway). New code uses it.
## D2552: reduce by argument count, its accumulator in the dense loop's state
date: 2026-11-03 · by: speed · issue: 0162
`%ArgumentCount()` (a leaf, also in the tier-0 loop's intrinsic hits) is the calling
frame's argument count before padding. `Array.prototype.reduce` and `reduceRight` take
`initialValue?` as a plain parameter and test `ArgumentCount() < 2`, instead of a
`...rest` array per call, and each runs in its own frame (the shared `Reduce` helper and
its second frame are gone; reduceRight has no dense part, as before). `%EachDense` mode 7
takes the starting accumulator as `thisArg` and keeps the running one in that state slot
(the callback's `this` is undefined, as the spec's); each result also goes to element 0
of a box Array, which reduce reads when the returned k is past its starting k. The box is
`ArrayCreate(0)`, also a tier-0 hit, made only for a non-empty array. wasm instructions
against main (wmeasure.py, n=5): tiny-reduce0 −46%, reduce1 −25%, reduce2 −23%,
tree-children (pico's 9b) −11%; the other 26 workloads within ±0.8% (json-date +0.8% at
n=7, noise level); size −70 B gzip.

Not done, measured: handling the callback's return on short arrays inside `run_hot`
(0154's follow-up). A wasm CPU profile of tree-children after this change puts all of the
`%EachDense` machinery (each_return, each_complete, each_element, each_finish,
each_dense) at ~5%, so moving it into the hot loop would save at most 2–3% for a copy of
`each_return`'s logic in hot.rs. The same profile: run_hot 26%, the collector 26%
(marking and compacting the live tree), closure creation ~6–8% (fclosure through `step`,
then `function_properties`' two define_own_property calls), species and ArrayCreate's
`length` define in `map`'s fixed cost.
## D2433: Keyed string hashing for the VM's own atoms and Map/Set
date: 2026-11-09 · by: strings · issue: 0163
docs/lessons-from-pico.md lesson 5: djs hashed strings with a fixed multiply-rotate hash,
so keys could be built offline to collide (2,048 ASCII strings with one hash take a few
birthday searches over a 32-bit state, the test does it in 0.1 s), and a script reading
them as JSON keys, computed property names or Map keys would spend quadratic time.
- **The hash:** SipHash-1-3 over UTF-16 code units, four to a 64-bit word, the unit count
  in the last word; never 0 (0 is "not cached"). `KeyedHasher`, `hash_latin1_with`,
  `hash_units_with`, `keyed_u64` (Map/Set keys that are numbers, BigInts, objects or
  symbols). The old `StrHasher` is gone.
- **Keys:** `Roots::HASH_KEY` (u64 at 104; Roots' minimum size is 112) is the VM's key; the
  base's atom index uses `BASE_HASH_KEY` (0), the same in every VM of the build (the
  prefix is immutable, D2064). The VM's key is `hash_key_for_seed(seed)` (one-way, so it
  is not the `Math.random` state a script can read back): `Vm::new`, `from_base` and
  `set_random_seed` set it with the seed, a restored image keeps its own. Deterministic
  mode passes its seed, so the same seed lays out the same heap; hosts seed from entropy
  otherwise (the Sandbox, pi). A host that never seeds gets the default seed's key.
- **Caches:** a string caches the hash of its level: below `BASE_END` the base hash (set
  at the base build, `prefix_hashes_to_base`), above it the keyed one. `str_eq`'s
  fast reject compares cached hashes only on the same side.
- **Lookups:** the VM's own atom index by the keyed hash, then the base's by the base
  hash, computed only when needed; a string of the prefix (an atom's own string pushed
  back as a key) tries the base first, whose hash it caches. An atom's own string is
  matched by identity before any hash compare. The VM's own atoms are always strings
  above the prefix (`intern` copies a prefix string), which cache their keyed hash. The
  VM's index stays at most 1/4 full below 1,024 buckets (1/2 above, pi's prelude), since
  every lookup of a base atom's name misses there first.
- **Rekeying** (`Heap::set_hash_key`, when the key changes): the cached hashes above the
  prefix are cleared, the VM's own atom index and every Map/Set index are rebuilt.
  Insertion order is the entries' order and does not change. The prefix is not written.
  `Vm::set_math_random_seed` (wasm `vm_set_math_random_seed`) reseeds `Math.random`
  alone: pi keys its VM from entropy before running its prelude outside deterministic
  mode and only reseeds `Math.random` per session; in deterministic mode it keys every
  session from its seed (layout = function of the seed, cold or warm start).
- **Natives:** `n_str("toJSON")` and `n_str("rawJSON")` return the atoms' strings instead
  of new ones, which the JSON natives looked up per value (two hashes per lookup now).
- **Measured** against main: native instructions on the perf workloads all within
  +0.31% (props), json-date −4.9%, json-indent −1.8%, json-replacer −1.5%; wasm
  (wmeasure, repeated) within ±0.9%. Not in the workloads: Map with int keys +4.3%
  (a SipHash per operation where a multiply-xorshift was), Map with string keys −2.4%
  (`collection_set` caches the key's hash), JSON.parse of 500 distinct keys +3.0%. pi
  (scripts/snapshot-research/wasm.ts, deterministic): containers +1.0 KB (ask 2.2 →
  3.2 KB, agent-loop 6.4 → 7.3 KB median), since each session rekeys the prelude's atoms
  and tables; outside deterministic mode no session rekeys.
- **Size:** module +453 B gzip (444,523 → 444,976 B), slim module +434 B (378,784 →
  379,218 B), both under their ceilings, which stay. A first version cost +4.5 KB; one
  copy of SipHash's rounds (`compress`, `finish64` out of line), one probe loop for both
  levels and every caller (the needle enum), no dyn iterators and dropping the old fixed
  hash brought it down.
- **Compatibility:** `vm::COMPAT_VERSION` 1 → 2: the hashes stored in images (string
  caches, the atom indexes, Map/Set indexes) are computed differently, which the layout
  and records hashes don't see (the SipHash constants are not top-level consts).
- **Not done:** the collision test covers strings; keys built against SipHash need the
  key. Atom ids and the base keep their layout.


## D2542: Slices: substrings that share a one-byte parent, copied out when they pin it
date: 2026-11-09 · by: heap · issue: 0165
Step 1 of 0165 (docs/lessons-from-pico.md 2.3). A new string cell kind `Slice` (22;
24 B: `LEN` and `HASH` where every string kind has them, `PARENT: Ref`, `OFF: u32`) is
the units `[OFF, OFF + LEN)` of a flat `Str8` (never an append buffer, never a slice).
- **Made by** `builtins::string::substring` (slice, substring, substr, trim, regexp
  matches and captures, the general split path), the fast one-byte `split`, and
  `json::parse` for string values without escapes (one-byte input only). Thresholds are
  constants in heap/string.rs: `SLICE_MIN_LEN` = 32 units and `SLICE_PARENT_MIN_LEN` = 256
  units of the string cut; a slice of a slice points at the root parent. Ropes are not
  sliced (split and the intrinsics flatten first).
- **Read** in place: `str_view`, `str_view_flat`, `str_chunks`, `char_code_at`, the
  concat leaf writers and `alloc_latin1_pieces` handle the kind, and `Heap::contiguous`
  (one kind read) replaces `flatten` where a reader only needs a view (regexp exec, the
  string intrinsics, iteration, JSON.stringify's quoting, number parsing, `indexOf`'s
  needle). `flatten` copies a slice once and turns the cell into a flattened `Rope` in
  place (the field offsets match), which atoms (`intern`), Map/Set keys and
  `str_data_offset` users get. `set_hash_key` clears slices' cached hashes (D2433); the
  hash goes over the units through the views, so it equals a copy's.
- **Collector**: a full collection marks a slice without its parent; after the mark, two
  walks over the marked slices sum, in each unreached parent's `HASH` field
  (`gc_bits::COUNTED`), the units its slices cover. A parent of at least
  `SLICE_KEEP_LEN` = 4096 units covered less than `1/SLICE_KEEP_SHARE` = 1/4 is kept for
  this collection and its slices flagged (`SLICE_DETACH`); right after it, each flagged
  slice is flattened, and a second mark-compact drops the parents (so a snapshot right
  after a collection does not carry them). Minor collections keep parents. The Cheney
  copy writes a slice as a flat `Str8`, so canonical images and hashes do not depend on
  slices (its to-space is uncapped for that).
- **Images**: `verify` checks `PARENT` is a `Str8` and the range fits; the JS reader reads
  slices; js/src/layout.generated.ts regenerated. The layout hash changes by itself.

Measured (work/scratch/0165/cases.ts, Sandbox with async host functions, against main):

| case | main | slices |
| --- | --- | --- |
| log-5mb: 5 MB log, `split("\n")`, filter | arena 12.0, peak 20.1 MiB | arena 7.2, peak 17.3 |
| log-1mb | arena 2.6, peak 4.4 | arena 1.6, peak 4.3 |
| log-wait: log-5mb's state in a snapshot | 11.70 MiB raw, 1.75 zstd | 6.89 raw, 0.98 zstd |
| parse: JSON.parse of 2,000 records with 1 KB bodies, snapshot | 4.45 raw, 0.05 zstd | 2.39 raw, 0.04 zstd |
| pmap-200, records | unchanged (no cutting) | |
| pi bench tool-heavy, warm (wasm pages) | 24 | 20 |

The perf workloads' peaks (mem-perturb n=8) are unchanged, and so are the pi bench
times. Instructions in wasm moved −1.4% to +2.8% between repeated runs on the same
builds (V8's code generation for the interpreter); natively (`djs run`, n=5) strings
−0.1%, push-pop +0.3%, json-date −0.9%.

Sweep of the constants (cases above; the workloads do not cut long strings): slices of
16 or 64 units, parents of 64 or 1024 units, and shares of 1/2, 1/4, 1/8 gave the same
numbers on log, pmap, parse and records. The share matters where slices pin a parent:
"snippets" (100 × 60 units of one 5 MB log and 100 × 20,000 of another, logs dropped)
snapshots at 1.93 MiB on main, 5.03 with 1/4 or 1/8 (the second log is 40% covered, so
kept), 1.93 with 1/2, whose peak rose from 11.1 to 21.3 MiB while copying 2 MB out. The
defaults stay as the coordinator set them.

**Size (trade, coordinator):** +1,956 B gzip (442,260 → 444,216; the module's ceiling
raised to 444,500) and slim +2,233 (377,381 → 379,614; ceiling 379,900): the kind in the
readers, the collector's walks, the canonical copy, JSON.parse's slicing. The parts
measured and dropped to get there: a sort-and-group of the slices by parent (+4.4 KB
gzip; the `COUNTED` bit and the parent's `HASH` field do it without a table), slicing
concatenation ropes (+0.2 KB), a separate unslice path (merged into `flatten`).

## D2543: A wasm stack overflow is reported as one
date: 2026-11-09 · by: heap · issue: 0166
djs-wasm exports its shadow stack pointer (`--export=__stack_pointer` in build.rs). A
frame that does not fit the 128 KiB stack (D2540) takes the pointer below 0, where it
wraps past the end of memory, and the first store traps as "memory access out of
bounds". `trapText` (js/src/trap.ts), used by the Sandbox's `guard` and the pi engine's
`trap`, reads the pointer first: above the memory's size it reports "stack overflow:
the engine's wasm stack is exhausted (…)" without calling the instance again (with the
pointer wrapped, `djs_panic_ptr` itself trapped and turned the report into a raw
RuntimeError); otherwise the panic message or the runtime's text as before. The kind
stays `trap` (the instance is dead); the pi engine's `dispose` no longer calls a trapped
instance. Checked on a build without the parser's nesting limit; js/test/trap.test.ts
covers the cases with a fake instance. Size +20 B raw.

## D2544: GC thresholds count their pages from the prefix's end
date: 2026-11-09 · by: heap · issue: 0166
`next_threshold` rounded `live + growth` up to whole 64 KiB pages from offset 0 (D1870),
so the builtin prefix's size moved every collection point (json-date's phase flips in
D2443/D2445). It now rounds `live - BASE_END + growth` and adds `BASE_END` back: a
smaller or larger prefix moves the threshold by exactly as much, and collections come
after the same allocations (unit test over young sizes and prefixes ±46 KB). The arena's
growth target (`Arena::set_target`) stays whole pages from offset 0, the threshold
rounded up, since that is where its memory is.

mem-perturb n=8 against main (mean MiB): json-indent 4.60 → 3.80, json-replacer 4.24 →
3.60, pi-like 1.60 → 1.40, json-date 5.16 → 5.00, props 1.10 → 1.00, strings 11.02 →
10.91, strbuild 14.60 → 14.80, jsonh 5.31 → 5.40, the rest equal. Wasm instructions n=5:
−1.4% to +1.0%. Rejected: the relative rounding with the target unrounded (strbuild
15.2, json-replacer 4.19), and no rounding at all (strings 12.10, json-replacer 4.60,
push-pop 1.25).

## D2545: Nesting limits re-measured: parser 550 stays, regexp 212
date: 2026-11-09 · by: heap · issue: 0166
Re-measured with the limits lifted (MAX_DEPTH 5000, regexp 2000) on the current main
(D2521's 16-word `push_out` was already in D2540's last measurement), deepest nesting
without a trap at 128 KiB, debug | release: blocks 611 | 798, trys 610 | 798, classes 77
| 90, functions 148 | 189, parentheses 248 | 285; capture groups 258 | 243, lookaheads
249 | 235. The 10% rule allows a parser MAX_DEPTH of at most ~554 (blocks and trys in
the debug build the gate tests), so 550 stays. The regexp parser's limit goes from 201
to 212: 211 nested groups, 10% below release's 235-lookahead limit; both builds reach
the limit's SyntaxError at 211 for every regexp construct.
## D2553: JSON.stringify builds a large result in its heap string
date: 2026-11-03 · by: speed2 · issue: 0162
The fast path's output buffer no longer holds the whole result. At a check point (after
a primitive, a Date or a container's opening) a buffer of 4096 units or more moves into
the result's heap string and starts again
(`Ser::spill`): appended in place while that string is still the arena's last cell
(`Heap::append_last_latin1`: the cell grows at the frontier, the units are written once),
else as a new piece; several pieces are joined into a rope at the end. 0116's objection
to writing in place (the fast path must never allocate in between, which nobody enforces)
does not apply: an allocation in between (a rope flattened, an index atom) only starts a
new piece. Two-byte output also starts a new piece at each spill (an in-place two-byte
append cost ~200 B gzip; such results end as ropes of 4096-unit pieces). The buffer's
own checks against the string length limit and the heap's room went: the buffer stays
small, and the heap's allocations apply both limits as the result grows.

Peak wasm memory (mem-perturb, 12 runs, mean MiB, main → now): json-date 5.14 → 3.50
(D2541's accepted +0.9 MiB is gone), json 5.80 → 5.40, jsonh 5.31 → 4.91, json-replacer
4.23 → 4.02, an 8 MB result of 80,000 records 34.2 → 25.9; json-indent, json-text,
pi-like, strings and a 4 MB string unchanged. Wasm instructions (default tiering, n = 5,
outliers re-run at n = 9) within ±0.8% on every workload and pico case; json-big −1.6%,
the pi bench −0.3%. Natively, json and json-text +0.1%. Size: within both ceilings
(slim 4 B under) after trimming: the first version was +890 B gzip; a check point at
each closing bracket (bounding a long run of them) and the out-of-line `Buf::alloc` went
for the last bytes, so a run of closing brackets stays in the buffer as before.

Measured and dropped:
- Long strings quoted in 4096-unit parts with check points between them (a pair kept
  whole): +200 B gzip, and a 4 MB string built by `repeat` peaked higher (13.9 → 15.8
  MiB; 0.5–2 MB strings 0.3–2.2 MiB lower). A long string goes into the buffer whole
  and moves out at the next check point.
- Reserving the arena for a long string's length before writing it: worse on small
  strings, no better on large ones.

## D2554: Array species and ArrayCreate shortcuts
date: 2026-11-03 · by: speed2 · issue: 0162
- `Heap::new_array_with_length`: ArrayCreate(len) sets the fresh array's length directly
  instead of through `define_own_property` and ArraySetLength's descriptor checks, which
  a fresh array always passes (`%ArraySpeciesCreate` fast entry and the ARRAY_CREATE
  intrinsic).
- `plain_species`: when `%Array.prototype%` and `%Array%` are frozen builtins (D2304),
  their `constructor` and `@@species` are the base's, so the four own-property lookups
  are skipped. The receiver checks before it are unchanged; without frozen builtins
  nothing changes. Tested in vm/frozen_tests.rs (the difftest corpus does not run with
  frozen builtins).

`[].map(f)` was 6.3k wasm instructions, about 10% in plain_species and 4% in the length
define. Wasm instructions against main (default tiering, n = 5): tiny-map0 −5.5%,
tiny-filter0 −5.6%, tiny-map1/2 −3.6/−3.3%, tiny-filter1/2 −4.0/−2.1%, tree-children
−1.7%, tree-filter −2.1%; every other workload and pico case within ±0.8% (props,
push-pop, tiny-reduce2 re-run at n = 9: −0.6%, +0.2%, −0.7%); pi bench −0.3%. Size
−88 B gzip.

## D2446: One constant pool per compilation unit
date: 2026-11-03 · by: bloat · issue: 0151
The functions of a compilation unit share one `ValueArray` of constants (`resolve::UnitPool`):
strings are shared by content, numbers by bits, as the re-emit meets each `push_const`
(indexes in emit order, so the first 256 get `push_const8`). The runs that must stay
contiguous and private, a tagged template site with its `n, cooked, raw` strings (the
site slot caches the template object) and the global declaration list, are appended as
they are emitted. The top level has a pool of its own: it runs once. Ranking the
constants by use before emitting saved 138 B of base bytecode for 2 KB of wasm (and
`slice::sort_by` another 6 KB; a `BTreeMap` for the dedupe cost 16 KB, now a small open
addressing table): not done.

The base image build drops what only dead code used (`image::prune_pools`, after a
collection): the prelude's installing code is a function of the same unit that dies
before the base is written. Entries keep their order, so every index only gets smaller
and `push_const8` still fits: the immediates are patched in place.
- **Bought:** base 217,872 → 207,920 B (−9.95 KB per VM; 396 pools, 13,568 B → one of
  371 entries, 2,984 B; bytecode +606 B for the indexes past 255); module gzip 444,487 →
  443,959 (−528), slim 379,910 → 379,601 (−309). User scripts get one pool cell per unit
  and per top level.
- **Neutral:** wasm instructions (n=3, all workloads) −0.8..+1.4%, re-measured n=7
  −2.5..+0.2%, warm equal.

## D2447: The base's atoms are renumbered without gaps
date: 2026-11-03 · by: bloat · issue: 0151
`forget_string_atoms` (D2253) used to leave a forgotten atom's entry `undefined`: 1,455 of
the base's 1,946 atom table entries were such holes, 11.6 KB in every VM. It now moves the
kept entries down (the well-known atoms keep their fixed ids), copies them into an entries
array of exactly their size and returns the map from old to new index, and the base build
renumbers every atom id the heap holds by it (`Heap::moved_atom`, which panics on a
forgotten atom still in use):
- `Heap::renumber_heap_atoms`: property table keys (hashed tables get new indexes),
  shape descriptor arrays (shared along a chain: each once), each transition list
  re-sorted by its children's new last keys, symbols' own atoms;
- `image::renumber_atoms`: template names, `FuncInfo` local names, the `Atom`
  immediates of the code (u32, patched in place), and the atom ids of global
  declaration lists in constant pools.
Atom ids are otherwise only held in Rust while code runs (the wire format's key tables,
JSON's key cache), never across the image build.
- **Bought:** base 207,920 → 196,280 B (−11.6 KB per VM; atom entries 15,584 → 3,944);
  module gzip 443,959 → 443,720, slim 379,601 → 379,392.
- **Neutral:** warm wasm instructions equal (json 625/626, props 7066/7071 M); cold
  −0.1..+1.5% (props +1.2% at n=7, the only one beyond noise: first-run tier-up of a
  module whose code moved by 249 bytes).

## D2448: A closure's template in its ELEMENTS field
date: 2026-11-03 · by: bloat · issue: 0151
Only arrays get dense elements (`dense_accepts`), so a closure (`ObjectClass::Function`)
keeps its `FuncTemplate` in the object's `ELEMENTS` field (`Heap::alloc_closure`,
`Heap::closure_template`), and its slots are its captures, then the home object
(`Function.slots()` is 0). A builtin's function object is 24 bytes instead of 32.

A first version checked the class in `Heap::elements`: warm wasm instructions arrays +1.3%,
pushpop +1.7%. Instead the template's first word is `NO_ELEMENTS`, always 0, where a
`ValueArray` has its `LEN`: every element path reads a closure's template as an empty
array, with no check, and none writes one (dense stores are for arrays only, and
`set_element_fast` declines for other classes before appending). To make room, `FLAGS` is
a `u16` (13 flags used) and `NATIVE`/`FAST` are `u8` (2 and 54 entries; a test keeps
both tables below 255); `FLAGS`, `NATIVE` and `FAST` share one word, which the hot
return path reads at once. The template stays 48 bytes. In debug builds `Heap::elements`
asserts that a non-null field is a `ValueArray`, or a template of a closure with
`NO_ELEMENTS` 0; the verifier checks both, and `dense_put` asserts it never runs on a
closure.
- **Bought:** base 196,280 → 189,288 B (−7.0 KB per VM; function objects 27,968 →
  20,976); every user closure 8 bytes smaller. Module gzip 443,720 → 443,746 (+26), slim
  379,378 → 379,290.
- **Speed:** wasm instructions (n=5, all workloads) −2.6..+0.2% cold (call1 −2.6%, calls
  −2.6%, pushpop −2.1%, push-pop −1.3%, pi-like −0.6%, arrays −0.8%); warm arrays
  9601/9649 and 9700/9498 (noise), pushpop 3667 → 3592, push-pop 3423 → 3373, forof and
  foreach equal, pi-like 4252 → 4228; pico's tree-children −4.7%. Peak memory equal.

## D2449: No direct native binding for thin builtins (L1, L2)
date: 2026-11-03 · by: bloat · issue: 0151
Measured on main ab09a05f (base 196,280 B) and dropped:
- L1, compiling calls of thin forwarding helpers as direct intrinsic calls: only 9
  prelude functions are pure `return %I(params)` / `return F(params)` forwards.
- L2, builtin function objects bound straight to a Rust entry with an argument map (as
  QuickJS's magic): 290 prelude functions are one `return` of a call, but their
  arguments are coercions and receiver checks written in JS (`%I(ToNumber(v))` 49,
  `%I(ThisMap(this), v)` 13), constants and `new.target` passed to shared helpers
  (`EveryOrSome(%ToObject(this), cb, t, true)`, `ErrorConstruct(new.target,
  "XPrototype", ...)`, `%BufferInfo(this, k)`). Those helpers can run user code, so a
  binding would need its own small argument interpreter next to the bytecode, with
  reentrancy, suspension, fuel, stack traces, `new` and `length`/`name` without a
  template: a second calling convention for ~10-13 KB per VM (after D2444-D2448 a thin
  builtin's template is 48 B + ~12 B of code), paid with new Rust in a module close to its
  size ceiling. Reopen only with new facts (a smaller binding, ceiling room, or many more
  pure forwards).

## D2450: The base's property table indexes at a load of three quarters
date: 2026-11-03 · by: bloat · issue: 0151
L5, measured on main 14c83b2d: the base has 83 dictionary property tables (14.9 KB) and 23
hash indexes (7.5 KB), sized at twice the table's capacity. The tables of builtin
prototypes and namespaces are full after `compact_for_image` and grow only when a script
adds to a builtin, which first copies the table and builds a normal index
(`grow_table`). So the image build gives them an index of `next_power_of_two(⌈4n/3⌉)`
buckets (`Heap::build_table_index_dense`, also after the atom renumbering of D2447): a
load of at most 3/4, always with an empty bucket for a miss to stop at.
- **Bought:** base 189,288 → 186,344 B (−2.9 KB; 19-entry tables 64 → 32 buckets,
  38-47 entries 128 → 64); module gzip −150.
- **Neutral:** wasm instructions (n=5, all workloads) −0.1..+0.5% cold apart from indexof
  (+0.7% at n=7), warm props 6997/6989, prop1 2260/2261, pi-like 4226/4232 M.
- **Not done:** a compact frozen form. The entries are 16 bytes (an atom, flags, a value)
  and the values are mostly distinct function objects; a shared key description is a
  shape chain, which costs a cell per key for one-off objects (D1831). Linear search up to
  12 entries would save 240 B more.

## D2352: djs-wasm leaves the dumper out by default (+32.4 KB gzip)
date: 2026-11-03 · by: portable · issue: 0167
`Vm::dump` (feature `dump` of djs-core, default on: the CLI and native embedders have it)
reaches djs-wasm only through djs-wasm's own feature `dump` (export `vm_dump`), which is
off by default. Measured on the `full` wasm build, gzip -9: 443,469 without, 475,863 with
(+32.4 KB, +93 KB raw), against a ceiling of 444,500. The first version cost +58 KB:
`BTreeMap`s, slice sorts, `format!`, substring search and str slicing (whose panic path
pulls `Debug` and Unicode tables) reached from the dump path, plus the registry's
prelude-functions pass, which compiles the prelude source (~66 KB gzip of text) to name
internal functions a dump refuses anyway (`registry::reachable_names` skips it). The
dumper now uses one open-addressing index of cells and atoms, the walk's insertion order,
`crate::text` (D2339), and the registry's naming walk a bitset and `crate::sort`; what is
left is its own logic. The gate's debug wasm build enables the feature, so the js tests
cover `Sandbox.dump`/`load`. The feature is image-neutral and outside SOURCE_HASH's feature
list: it only reads the heap. A wasm host that wants dumps without the module cost uses
the host-side dumper (D2354).

## D2353: Dumps are Scripts: format version 1 and the promise that later builds load it
date: 2026-11-03 · by: portable · issue: 0167
A dump (docs/functions-as-code.md) is a JavaScript Script whose first line is
`// djs-dump 1`. It recreates the selected bindings as top-level declarations (`let`/
`const` for the global lexical record, `var` or the function declaration for the global
object), using only the language, standard builtins and helpers it defines on a temporary
`globalThis.$dump` that it deletes at the end. Builtins are referenced by their D2371 paths
on the base image (cells below BASE_END are builtins at the same offsets in every VM of
the base), as expressions or through `$dump.builtin(path)` for realm roots with no global;
builtins the session changed are written as the code that changes them. Closures are
their verbatim source made by one factory per group of closures that share captured
variables (not rewritten into environment objects: no parser, the text stays the user's);
values on cycles are patched after the data. Values that are live (pending promises,
activations, proxies, host functions not re-bound by name, classes for now) are refused
with their path. **Promise:** every later djs loads a version 1 dump and recreates the
same bindings; tests/dump/v1.js is loaded by a test in every gate run, and a format change
gets a new version and keeps the old fixture. No reader code is kept for it (D024): a
dump is loaded by running it.

## D2354: A wasm host dumps from the heap bytes in TypeScript; the Rust dumper is the reference
date: 2026-11-03 · by: portable · issue: 0167
Measured on the same cases (js/proto/compare.ts): the TypeScript prototype over the heap
bytes (js/proto/dump.ts, the reader's approach) writes the same text as `Vm::dump` (7 of
8 round-trip cases byte for byte, the same refusals, 9.7 MB identical for 50,000 records),
runs twice as fast as the Rust dumper in release wasm on node 25 (10k records: 52 vs
99 ms) and costs 13.7 KB gzip of JS instead of 32.4 KB gzip of wasm (D2352). The Rust
dumper stays the reference: it reads the heap through the engine's accessors, serves the
CLI and native embedders, and its tests define the behavior. Recommended for wasm hosts
(pi): the TypeScript dumper, once (1) layout.generated.ts exports the engine facts it now
copies by hand (function flags, intrinsic numbering, object-class slots, lazy function
properties, collection and regexp encodings), (2) it finds the globals a function reads
from the two opcodes instead of a source scan and keeps errors' stack, and (3) a gate step
requires both dumpers to write the same text on the same cases. Until then it stays a
prototype under js/proto, outside the package.

## D2355: A dump is checked by compiling it: every function keeps the bindings it read
date: 2026-11-03 · by: portable · issue: 0168
0168: a root whose value was itself a closure of a scope group (`const blah2 =
blah(2)`) was written in place as its bare text (`() => a`), so the dump loaded a closure
reading a global `a`. The fix sends such roots through their group's factory; the guard
against the whole class of bug is a checker at the end of `Vm::dump`: it compiles the
dump (lines that write no function cut to `let name;` or dropped: data cannot change
name resolution) into the private copy of the base heap the dumper already holds, and for
every function node compares the free names of the original template with those of the
template of the same text in the dump (captures, and globals read or written by the
function or a nested one, from the bytecode). A captured name must be captured in the dump
too, or be read as a global that is a root holding the variable's box or the root or host
global of that name holding its current value; a global must not be captured (a factory
variable hiding it), and must exist after loading if it existed before (a root, a host
global, a builtin global). Otherwise the dump is refused with the function's path, as is a
dump that does not compile. It needs no JS parser of its own (the engine's compiler
resolves the names) and costs a compile of the cut-down dump plus ~6 KB raw (~3 KB gzip)
in the wasm module. Functions of identical text and free names are compared once.

## D2560: A waiting session persists as a continuation dump; heap images stay inside a process
date: 2026-11-03 · by: coordinator (owner direction) · issue: 0169
Persistence is a format 2 dump (docs/continuation-dumps.md): a Script, like version 1
(D2353), that also recreates suspended activations, open host calls and in-flight promises.
Every later djs loads it and continues the run, or refuses it with a reason; it never
misreads. Heap images remain for what stays within one process: the base image, pi's
prelude image as a startup cache (D1495), the stress and determinism checks (D1720), and the
transport between module variants (D2564). Snapshot containers, session images, reference
compression and the APIs on them (`.snapshot`, `Sandbox.bytes`/`fromBytes`, pi's
`snapshot`/`resumeVm`) are retired by 0177 once dumps cover the harvested corpus; this
supersedes D2260, D2261, D2262, D2066, D2400 and D1890 as persistence formats from then on.
The owner's reason: images do not work as pi's persistence (tied to the build, opaque), and
codemode needs a form that survives upgrades. ARCH's G4 and G5 are restated accordingly.

## D2561: Dumps are taken only at idle points
date: 2026-11-03 · by: coordinator (owner direction) · issue: 0169
A dump is taken when the VM is idle: no queued jobs, no frames on the stack, no parked sync
host call. So fuel debt, native frames and half-run instructions never need a portable form.
The owner's reason: only externally visible effects matter, and those are tool calls, at
which a run is idle anyway; whatever ran since the last result reruns deterministically
(D1720). A host that must persist a computing run gives it fuel to reach an idle point, or
kills it. Suspending anywhere within a process (fuel slices) is unchanged.

## D2562: In-flight builtin state is re-enacted; a few privileged loader primitives
date: 2026-11-03 · by: coordinator · issue: 0171
A format 2 dump writes in-flight builtin state as the public calls that recreate it, where the
spec makes the result indistinguishable: promise combinators called again with settled
elements' values, reactions added again in order, iterators made and advanced. What plain
JavaScript cannot express goes through primitives the loader passes in on `$dump`, absent from
an ordinary Script: `resume` (a suspended activation at a named site), `cell` (a captured
variable's cell), `pending` (an open host call), `handle`, `private` (private fields and
brands) and `TDZ`. Rejected: describing builtin records by versioned schemas (0141), which
needs a schema per closure kind and record and a migration per change.

## D2563: The compiler's suspension maps are a pinned contract
date: 2026-11-03 · by: coordinator · issue: 0169
Each `await`/`yield` is named by kind and ordinal in its function's text (`await#2`), and the
compiler describes it on demand by compiling again: the live bindings (name and declaring
scope, hidden locals by stable name), the operand roles below the awaited value
(docs/portable-snapshots.md §3.5, emitted by the compiler where it emits the construct) and the
enclosing loops with their iterator records. A test pins the maps of every await in the corpus
and the e2e async tests; a diff is accepted only as a decision (older dumps stopped there
refuse, or a mapping is added).

## D2564: One dumper, in Rust; the TypeScript dumper is dropped
date: 2026-11-03 · by: coordinator (owner agreed) · issue: 0175
Supersedes D2354. With continuations a second dumper would repeat every role, re-enactment
and class case under a byte-identity check, and read generator frames, reaction lists and the
host call table; suspension maps come from the compiler in wasm anyway, and the CLI and REPL
need the Rust dumper. js/proto/dump.ts is retired. For wasm hosts the same code ships either
in the main module (feature `dump` on, the size ceiling raised) or in a second module of the
same build that the host loads to persist and resume, with images as the transport inside the
process (the feature is image-neutral, D2352). 0177 measures both after the image persistence
code is gone; the plan leans to the second module.

## D2565: djs builds pi's future prelude shape in its own adapter first
date: 2026-11-03 · by: coordinator (owner direction) · issue: 0176
pi's prelude keeps in-flight state (pending resolve/reject per call id, `nextId`, `finished`,
store writes, `run()`'s reactions) that a dump would have to carry. The target shape: tools
are engine async host functions (D1374), the body's promise is a host handle, `finished` and
the writes live in the host, so the prelude holds no in-flight state and is rerun on load
(D2262). djs builds it as tests/pi/prelude-next.ts and an adapter mode, with pi's suite run
against both; tests/pi/prelude-source.ts stays verbatim. pi itself is not changed yet.

## D2590: Corpus tool calls are idle points: `host.later`
date: 2026-11-03 · by: infra · issue: 0173
The difftest corpus's fake tools resolved through promise jobs, so a djs run never became
idle and a dump-at-every-idle-point mode (0172, D2561) would have had nothing to test.
The corpus prelude's `host` now has `later(value, rejected)`. In djs it is an async host
function (D1374): the runner settles the oldest pending call with its value (or rejects
with it) whenever `run` returns `Idle`, one per idle point, so a tool's answer arrives
after every job, as a real tool's does. Under qjs it is `Promise.resolve`/`reject`. The
prelude's `echo`/`fail` and the harvested cases' fakes (tools and `models`, rewritten
in place with `build.py --refresh`) answer only through it, and the prelude exposes it as
a non-enumerable global `later`; a host without `later` gets the Promise versions. The
harvested fakes are no longer async functions: `later` returns the promise directly, so
qjs takes the same number of jobs as before and every `.expected` file stayed as it was.
The two engines can now differ only for a case whose output depends on a tool answer
racing other promise jobs; no corpus case does. The runner reports idle points per case
(`--idle-points`; at 0173 1,194 in 294 of the 549 cases, 1,164 in 288 harvested ones),
and a stress variant must pass as many idle points as the plain run.
## D2600: The CLI's `tools`: a Proxy that makes one async host function per name
date: 2026-11-03 · by: repl · issue: 0174
`djs repl` (always) and `djs run --tools` define a global `tools`. It is a Proxy over an
object of made functions: reading a new name calls a sync host function (`make`, id 2)
that creates an async host function (D1374) with the next id from 16 and that name, and
the Proxy keeps it (`then` and symbols give `undefined`, so `await tools` is harmless). So
an open call is an ordinary pending async host call with its own function: `.pending`
and a later format 2 dump name it `tools.read`, and ids and names live in the heap, so
the REPL reads them back after a restore. The alternative, one generic `call(name, args)`
host function, would have put the tool's name into the arguments of every pending call.
The cost: making a function is a host call, so call ids count it (the first read of a
name takes an id). The REPL's waiting inputs need the input's result when `run` stops
`Idle` with calls open; `Vm::outcome()` (additive) gives the entry call's value then.

## D2601: Scripted tool answers: by arguments, else in order
date: 2026-11-03 · by: repl · issue: 0174
`--tools answers.json` maps tool names to lists of `{"value"}`/`{"error"}` entries,
optionally with `"args"`. A call takes the first entry whose `args` equal its arguments
as JSON (reusable), else the next entry without `args` (the last repeats). Answers are
delivered one at a time, at idle points (no jobs, only tool calls open), oldest open call
first, so every answered call is an idle point: the points `--dump-after K` counts.
Answering by arguments keeps a run split by a dump and `resume` answering the same way as
an uninterrupted one, with no record of what was consumed.
## D2570: Suspension maps from a role stack the parser keeps
date: 2026-11-03 · by: bloat · issue: 0169
How D2563 is built. The parser keeps, per function, a stack of operand roles: a construct
that leaves values on the operand stack while it parses a nested expression pushes their roles
where it emits them (`prepare_call` after `get_method`: `call.callee call.this`; each argument
after it is parsed; `binary.left` around the right operand; a for-of's `iterator next` after
its setup) and pops them where it consumes them. Every `await`/`yield` records the stack, its
innermost scope, its source position and the phase-1 index of its instruction (moved along by
loop rotation). Resolution finds the instruction in the final code, takes the resume pc and
checks the role count against the operand depth from `stack_depths`: a mismatch, or a role
`unsupported:<construct>` on the stack, makes the site unsupported, so a construct that is not
instrumented is reported, never mislabeled.

- **API:** `compiler::suspension_maps(heap, src, opts)` compiles `src` again with recording on
  (the templates are garbage) and returns a `FunctionMap` (name, span) per function with sites.
  A `Site` has its name (`<kind>#N`, numbered per kind in source order: `await`, `yield`,
  `yield.await`/`yield.return` for an async generator's yield, `return.await`,
  `for-await.next`/`for-await.close`, `await-using`, `yield*`), its position from the
  function's start, the resume pc, its operand roles bottom first, and every argument and
  local slot as a `Slot`: a binding (name and its declaring scope's position from the
  function's start), a hidden local by name (`<state>`, `this`, `<home>`…), an argument no
  binding lives in, or nothing (a binding whose scope does not enclose the site).
- **Vocabulary** (docs/portable-snapshots.md §3.5, where djs's lowering keeps more values):
  `call.callee`, `call.this`, `call.arg[i]`, `call.args` (the array after a spread),
  `new.callee`, `new.arg[i]`, `new.args`, `super.callee`, `super.arg[i]`, `super.args`,
  `member.object` (`o[await k]`), `super.this`, `super.base`, `object.literal`,
  `object.key`, `array.element[i]` (djs keeps up to 256 leading elements on the stack) then
  `array.literal`, `binary.left`, `assign.object`, `assign.key`, `assign.old`,
  `template.part[i]` (djs keeps the parts and joins them at the end: no `template.prefix`),
  `import.specifier`, `intrinsic.arg[i]`, and the loop records `for-of.iterator`,
  `for-of.next`, `for-in.iterator`, `for-await.iterator`, `for-await.next`.
- **Unsupported for now:** `switch` case tests (the discriminant), `finally` blocks (the
  pending completion), `using` regions (the disposal stack), destructuring, class
  definitions (heritage, computed keys, everything the class keeps), `yield*`, and the
  suspensions of closing an async iterator (a for-await left early, a generator's return
  path).
- **Cost:** nothing in templates or images. The hooks exist only with the `dump` feature
  (default on in djs-core, off in the shipped module), where recording is off unless
  requested; the pin test checks that a recording compile produces the same code. Without
  the feature the hooks are empty: the shipped module +34 B gzip, the slim one +88 B
  (argument kinds and positions passed to now-empty hooks).
- **Pins:** `compiler::smap_tests::pinned_maps` writes tests/smap/{codemode,
  codemode-harvest,e2e}/*.smap (a slot table per function, one line per site with its roles
  and the slots out of scope there) and tests/smap/summary.txt (sites by kind, by role
  sequence, unsupported); `DJS_UPDATE_SMAPS=1` accepts a change.
## D2580: While a dump loads, static initializers skip static fields and blocks
date: 2026-11-04 · by: portable · issue: 0170
A dump writes a class as its verbatim text, so loading evaluates the class again; its static
field initializers and static blocks would run a second time (`static { registry.push(this)
}` pushes twice) although their effects are already in the dumped state. The compiler now
ends a class's static initializer's private-method installation with `jump_if_loading`
(a new branch op) to the initializer's return: while `Vm::load_dump` runs (`Heap::
loading_dump`, a transient flag, never in an image) static private methods and accessors
are installed, static fields and blocks are not; the dump restores the static fields'
values itself (D2582). Outside a load the op falls through. Measured on the default wasm
module: 443,460 -> 443,581 B gzip (+121; +187 raw: the op handler and two emission sites);
one more instruction (1 fuel) per class definition with static elements; the class bytecode
snapshot changed, and OPS_HASH with it (images key on it). Rejected: skipping the
initializer's call (static private methods would then be missing, and their closures live
only in the initializer's hidden bindings), and re-running statics in an empty scope
(silent double effects on builtins and the host).

## D2581: Format 2 is a Script run by `Vm::load_dump` with the loader's `$dump`
date: 2026-11-04 · by: portable · issue: 0170
`Vm::load_dump(text, &LoadOptions { fuel })` reads the header: `// djs-dump 1` runs as a
plain Script (version 1 makes its own `$dump`, D2353; tests/dump/v1.js keeps loading),
`// djs-dump 2` gets the loader's `$dump`, other versions are refused. The `$dump` handover:
a global property (not writable, not enumerable, configurable) holding an ordinary object
with the primitives (`private` now; 0171 adds `resume`, `cell`, `pending`, `handle`,
`TDZ`), defined before the Script runs and deleted after it and the jobs it queued; the
dump adds its helpers (`$dump.bytes`, `$dump.keep`, ...) to it and no longer deletes it.
The primitives are host functions with ids the loader reserves (0xFFFF_FF00 and up,
which embedders must not use), served only by the loader's own run loop: a dump run as an
ordinary Script has no `$dump` at all, a primitive that escapes the load parks the VM on
a host call its embedder does not know, and a dump that calls any other host function
fails the load. Every dump the dumper writes is format 2. Exposed as djs-wasm's
`vm_load_dump` (feature `dump`), `Sandbox.load` and the REPL's `.restore`. Rejected: wrapping
the dump in a function with a `$dump` parameter (its top-level declarations would no longer
be globals) and a global lexical binding (removing one is not a JavaScript operation).

## D2582: Classes and private fields in dumps
date: 2026-11-04 · by: portable · issue: 0170
A class is written as its verbatim text where a function would be (in place, or in its
scope group's factory). Its constructor, methods, accessors, field initializers and private
methods join one scope group, since one text makes them; members are found after discovery
as the closures of methods, getters and setters (by kind) whose text lies inside the
class's and that sit on the class or its prototype, plus the hidden closures they hold
(`<fields N>`, `<#m>`), and get their locations from the class. After the text: a
`$dump.keep(holder, keys)` helper per class and prototype (deletes members deleted since,
throws if the text no longer makes a member), static fields and later properties by
`Object.defineProperty`, the name, prototypes, freezing. A derived class's heritage path is
bound to the parent it has now by an arrow parameter
(`((Base) => class C extends Base {...})(parent)`), refused if the class's code uses a
variable of that name, skipped when the parent is the builtin of that path; heritage
expressions other than a name or a path are refused, as are decorated classes. Private
fields and brands go through `$dump.private(obj, C, "#x"[, value])`: the loader finds the
recreated class's private symbol, and a private method's or accessor's closures, through
the variables named `#x` and `<#x>`/`<get #x>`/`<set #x>` that the class's closures capture
(a private name no closure captures can no longer be read; such fields are refused).
Static private methods are installed by the class's evaluation (D2580). Functions that use
a class's private scope but are not made by its text (a closure a method returned) are
refused; methods that use `super` are written in their home object's literal or refused.
Residual: computed keys and heritage paths are evaluated again at load. Cost on the
`dump` feature (default wasm build + `dump`): 479,059 -> 491,966 B gzip (+12.9 KB with the
loader). Coverage: no harvested case and 10 of the 2,627 private scripts define classes;
9 of those dump, load and compare equal.

## D2583: The format 2 promise, concretely
date: 2026-11-04 · by: portable · issue: 0171
What D2560 promises for format 2 (`// djs-dump 2`), as the loader makes it checkable: a
format 2 dump is a Script whose only privileged operations are the calls of `$dump`'s
primitives (`private`, `resume`, `cell`, `pending`, `handle`, `issued`, and the value
`TDZ`) with the argument shapes of docs/continuation-dumps.md. Every primitive checks what
it is given against this build (the suspension maps of D2563, the class's private names)
and throws a TypeError with the reason on any difference, which fails the load: a later
build loads the dump and continues, or refuses it; it never fills a slot it cannot name.
After the Script and the jobs it queued the VM is idle and `Vm::load_dump` returns its
open calls and the handles. Golden fixtures (tests/dump/v2/*.src.js with a hand-written
`*.dump.js` at an idle point) load in every gate run, and their continued output must
equal the uninterrupted run's; a change that makes one refuse is a format decision.

## D2584: `$dump.resume` builds the frame from the map of the dump's own text
date: 2026-11-04 · by: portable · issue: 0171
`resume(fn, {at, awaiting, this, args, bindings, operands, loops, queue})` takes the
suspension maps by compiling the dump's text again (once per load, with recording on) and
the map whose span is `fn`'s; `fn` must come from that text (same script), so slots and
the resume pc are exactly those of `fn`'s code, and the exporter and the loader only share
names. The frame is what a suspension saves (callee, `this`, arguments padded to the
parameter count, a header with `argc`, locals, operands), in a Generator object made as
`generator_start`/`async_start` make it, AWAITING at an await (subscribed to `awaiting`
with the activation as handler, as `%AsyncAwait` does) or SUSPENDED at a yield. Bindings
are matched by name, or `name@scope` when shadowed; hidden locals are made by the loader
(`<state>`, `this`, `new.target`, `<active>`, `<home>`) or given (`arguments`); a binding
a closure captures lives in a Box, so the map's sites now say which locals hold one
(`Site::boxed`, from the compiler's storage; the pins are unchanged), and `$dump.cell(() =>
x)` hands the Box of the one variable an arrow captures to `resume`, also in its TDZ.
Operands are `[role, value]` pairs bottom first (an object cannot repeat `call.callee` for
nested calls), loops `{kind, iterator, next}` outermost first. Refused for now: for-in
loops (the key iterator is internal), and every site the map calls unsupported (finally,
using, destructuring, yield*). Cost on the `dump` module (default wasm build + `dump`,
gzip -9): 498,277 -> 507,534 B (+9.3 KB, +25 KB raw, the loader proper; the recording
compiler was in it already); the default and slim modules are unchanged (everything is
behind the feature).

## D2585: A load's open calls, handles and call ids
date: 2026-11-04 · by: portable · issue: 0171
`$dump.pending(id, name, args)` adds an async host call with the host's own id and a fresh
promise; the host maps `name` to its function id through `LoadOptions::host_fn` (an
unknown name refuses), so a dump names calls the way the host knows them (`tools.read`),
not by build-local ids; the call is marked reported (`new_host_calls` does not return it
again). The primitives are host calls themselves: while a dump loads their ids come from
0x8000_0000 up, above every id a dump may name (positive int32s), and afterwards the
counter is set to `$dump.issued(n)` if given, else to what it was before the load, and
never at or below an open call's id. `$dump.handle(name, v)` makes a heap handle;
`Vm::load_dump` returns `Loaded { calls: [OpenCall {id, fn_id, name}], handles: [(name,
Handle)] }` and the host drops the handles.

## D2602: The CLI's host globals `later` and `print`
date: 2026-11-03 · by: repl · issue: 0174
`djs run`, `djs resume` and `djs repl` define, next to `console` and `tools`, the two host
functions of the loader's fixtures and the corpus's idle points: `later(v, rejected)`
(async, id 3), answered with `v` at the next idle point (rejected with it if `rejected`),
and `print(...)` (sync, printed like `console.log`). So the format 2 fixtures and corpus
programs run and resume in the CLI as in the core's tests, and `later` gives hand-written
scripts idle points without an answers file. `run` now always defines `tools` too; a tool
call with no answer is the same exit code 1 as before.

## D2603: Resuming dumps in the CLI
date: 2026-11-03 · by: repl · issue: 0174
`djs resume dump.js` and the REPL's `.restore dump.js` load a dump into a fresh VM with
`Vm::load_dump`. The loader asks the host for the function id of each open call's name:
`tools.<name>` gets the tool's id from the registry (D2600; the Proxy's `make` reuses it,
so a restored call and a new call of the same tool agree), `later` its id, anything else
refuses the load. After the load the REPL does not read tool names back from the heap (a
tool of an open call may have no function there yet). The handles the dump names become
the REPL's waiting inputs, labelled by name (`[run] value`); `resume` drops them and runs
to the end like `run`. Dumping a waiting session (`.dump`, `.dump-every`,
`run --dump-after`, `run --dump-every-idle`) refuses with a message naming the missing
exporter (0172) until it exists.
## D2610: The exporter writes what is in flight as the calls that make it
date: 2026-11-04 · by: portable · issue: 0172
`Vm::dump` writes format 2 with whatever is in flight when the VM is idle (D2561; refused
otherwise, also with a delivered host result not applied yet). The roots are the open
async host calls (first, in table order, `$dump.pending(id, name, args)`), the host's
handles (`DumpOptions::handles`, last, `$dump.handle`) and the globals; the host names its
functions (`DumpOptions::host_fn_name`, the inverse of `LoadOptions::host_fn`), which
also writes host functions held as values (`$dump.host(name, isAsync)`, a loader
primitive). A pending promise is written by what made it (an open call, an activation,
reaction `j` of another promise, a combinator, `new Promise` handing out its resolving
functions, else `new Promise(() => {})`), and each promise's reactions are added in their
original order (activations by `$dump.resume`, `then`/`catch`/`finally` called again,
combinators called again with the settled elements in place); a dependency that would
change that order refuses the dump. Promises that data holds are patched into it at the
end, so an activation can need that data first. Activations are read by their function's
suspension map, which the exporter gets by compiling the function's script again into its
scratch copy of the base with the options it was compiled with: Script cells now keep
them (`Script::OPTIONS`, the goal and TypeScript/decorator flags; `Script::PARAMS`, an
async body's parameters), so a TypeScript or async-body script maps exactly (immediates
are fixed width, so pcs agree across heaps). The layout changed (images key on it); cost
in the default module +203 B gzip (443,610 -> 443,813: the two fields and the params
string; `join` was avoided, its generic copy alone is 1.3 KB). Rejected: guessing the
options (a wrong guess would name the wrong slots, never refuse), describing reaction
records by schema (D2562).

## D2611: Variables of one name in several scopes get nested factories
date: 2026-11-04 · by: portable · issue: 0172
A scope group (closures and the variables they share, D2353) wrote every variable in one
factory, so two variables of the same name (a loop's per-iteration bindings captured by
the closures of each iteration, as in the corpus prelude's `tools`) were refused. Now each
variable whose name is not unique in the group lives in a nested factory of its own, one
per set of closures that capture it; those sets must nest or be disjoint (else refused),
other variables go to the deepest factory whose closures include all of theirs, and a
closure is written in the deepest factory that holds it. Nested factories return their
closures and spread into their parent's returned object; keys stay unique in the group.
Cells an activation shares are returned as `$cN`; a variable in its TDZ is declared after
its factory's `return`, so it stays uninitialized and closures over it are written.
Classes in a nested factory are refused.

## D2612: Re-enactments of iterators and for-await state
date: 2026-11-04 · by: portable · issue: 0172
Array iterators and Map/Set iterators are made on their target and advanced by `next()`
(`$dump.advance`; nothing user-visible runs), string iterators by code points; a Map or
Set that only an iterator holds is made again from its live entries. The iterator a `for
await` over a sync iterable makes (CreateAsyncFromSyncIterator, never visible to user
code) is replaced by the dump's own `$dump.fromSync(it, next)`, whose `next`/`return`/
`throw` step like 27.1.6 (PromiseResolve of the value, one `then`, closing the sync
iterator on a rejection), and its in-flight reactions by the equivalent handlers; job
counts are the same. A generator that has not started is called again
(`Reflect.apply(fn, this, args)`, only when its parameter list runs no code: no defaults,
no destructuring); a finished one is a new finished generator with its prototype.
Arguments objects are made by a strict function's `arguments` when they are as a call
made them. Proxies are `new Proxy(target, handler)` (revoked ones refused). Rejected:
loader primitives for these (they would put builtin state shapes into the format).

## D2613: The gate dumps at every idle point of the corpus
date: 2026-11-04 · by: portable · issue: 0172
difftest `--dump-every-idle` dumps the VM at every idle point (the corpus prelude's state
included, its `api` as a handle), loads the dump into a fresh VM and continues there; the
transcript must equal the plain run's and a refused dump fails the case. It runs in the
gate's main difftest step: 1,194 idle points in 294 cases at 0172, about 15 CPU-s in a
debug build. e2e gets a fourth mode, dump, in its default modes: each case's final state is
dumped, loaded and dumped again twice; the second round must reproduce itself (the first
may order globals differently); refusals are counted (52 round-trip, 11 refuse at 0172:
decorators, private names outside their class, iterator helpers, weak collections).
`vm::dump_harvest::idle_*` count the same at every idle point of the harvested cases and of
private scripts (counts only): 1,164 and 5,257 idle points at 0172, all dumped, loaded and
equal.

## D2614: The dumper's size in the dump module
date: 2026-11-04 · by: portable · issue: 0172
Continuations cost the `dump` module (default wasm build + `dump`, gzip -9, same toolchain)
507,129 -> 537,056 B (+29.9 KB, +93 KB raw): the in-flight discovery and emission, nested
factories, site comments, and the helpers' JavaScript (`fromSync`, `advance`, `spent`).
Accepted for now: whether the dumper ships in the main module or in a second module of the
same build is 0177's measurement (D2564), and the plan leans to the second module, where
the main module pays nothing. Trimming is a follow-up.

## D2615: A dump holding TypeScript functions is TypeScript
date: 2026-11-04 · by: portable · issue: 0172
Functions are written as their source text, and a function of a TypeScript script has
TypeScript text (types, `as`, enums), which a JavaScript Script does not compile. When a
dump writes such a function its first line is `// djs-dump 2 typescript: ...` and the
loader (and the exporter's checker, and the loader's suspension maps) compiles it with
the TypeScript grammar; the text stays verbatim, so `toString()`, enums and parameter
properties behave as before, and the maps agree. Residual: JavaScript functions in the
same dump are parsed as TypeScript too, which reads a few JavaScript forms differently
(`a < b > (c)` is a generic call there); the checker does not see that. Rejected: erasing
the types (`strip_types` keeps positions) — it changes `toString()` and cannot erase enums
or parameter properties.

## D2616: Where activations stopped is reported, not written as comments
date: 2026-11-04 · by: portable · issue: 0172
The owner decided that the dump format carries no comments (0180 moves the existing ones
out and adds a separate human-readable explanation). So the exporter writes no comment
lines for what is in flight: `Vm::dump_report(opts, names) -> Dumped { text,
activations: [ResumedSite { loc, function, site, file, line, col, source_line }],
open_calls, pending_promises }` gives a host or a reader where each resumed activation
stopped (`Vm::dump` is its text). This replaces 0172's first take, a comment block with a
caret above every `$dump.resume`, and its "In flight" header line.


## D2640: The dump format marker is the directive `"djs-dump 2";`
date: 2026-11-04 · by: nocomment · issue: 0180
Dumps have no comments (D2641), so the format marker cannot be the comment `// djs-dump
2` either. A format 2 dump's first statement is the directive prologue `"djs-dump 2";`
(`"djs-dump 2 typescript";` for D2615's TypeScript dumps), on the first line. The loader
reads it as text before it compiles (it has to pick the grammar), so a marker the dump
evaluates, `$dump.version(2)`, would still need the text check plus a primitive that
repeats it; a directive is inert when the Script runs (an unknown directive does
nothing), and a format 2 dump run as a plain Script still fails at its first `$dump`.
The version number stays 2: only the marker's spelling changed. Version 1 dumps keep
their comment `// djs-dump 1` and still load (D2353, tests/dump/v1.js); the comment form
of any other version is "not a djs dump", including format 2 dumps of builds before 0180
(D024: nothing was promised across builds for format 2 before 0178). `djs repl`'s
`.restore`, `djs resume` and `Sandbox.load` take a file for a dump when it starts with
either marker. Rejected: `$dump.version(2)` (above), a new version number (nothing
semantic changed), `void "..."` or a global (not inert).

## D2641: Dumps carry no comments; `djs resume --explain` explains them
date: 2026-11-04 · by: nocomment · issue: 0180
The owner's call: the JavaScript dump format has no comments. The exporter no longer
writes the header lines (bindings, how to load, host globals), the section comments
("Builtins the session changed.", "Links that close cycles, and attributes.") or the
`<names> sharing <vars>` line above each factory; with D2616 that is every comment it
wrote. The order of the text says what the sections said (helpers, roots, builtin
patches, cycle links and attributes). Only the functions' own source text can contain
comments. For a reader, `djs resume --explain dump.js` (CLI only: no djs-core API and no
wasm size) loads the dump into a fresh VM and prints its open calls, the activations
waiting on each and the other suspended ones, each with where it stopped in the dump
(`dump.js:LINE:COL`), the source line with a caret under the `await`/`yield`, and its
bindings; the REPL's `.bt` prints the same caret lines. Positions are in the dump, the
file the reader has. Hosts that write dumps get the original positions from
`Vm::dump_report` (D2616). The hand-written fixtures in tests/dump/v2 lost their
comments too: the test names each one's idle point, and its program's header comment
says what the dump holds there.
## D2617: A codemode body is dumped as the async function it runs as
date: 2026-11-04 · by: portable · issue: 0172
Every pi run is compiled as a codemode async body (`Goal::AsyncBody`), whose function has
no source text of its own (its span is the whole script; `toString()` prints native
code), so the exporter refused the run's own activation. The exporter now writes such a
function as `async function (p1, p2) {\n<body>\n}` (the parameters from
`Script::PARAMS`) and resumes that; sites are named by order, so `await#N` is the same
site. Since the loader reads the frame by the wrapper's map, the exporter compiles the
wrapper's maps too (as the dump's goal, with the body's TypeScript options) and takes
the wrapper's site, after checking that its kind, parameter and local slots (by name),
boxes and operand roles equal the body's; else it refuses the body ("differs from the
async function the dump writes for it": a body that reads `arguments`, which is a
global there and the call's arguments in a function). Binding scopes are the wrapper's
(offsets from its start), the reported position stays the body's. The body's own
`Function.prototype` prototype is not checked (only the run holds the function).
`Function.prototype.toString` of a body is unchanged. Rejected: mapping scope offsets
by the header's length (fragile where the maps differ anyway).

## D2618: A script binding with a host name is the script's
date: 2026-11-04 · by: portable · issue: 0172
`DumpOptions::host` names the host's globals, which the loading VM defines again. A
global lexical binding of such a name (`const tools = {...}`) is the script's own and
hides the host's global object property, so it is dumped like any binding; only global
object properties of host names are taken as the host's values. The checker (D2355's
free-names check) refuses a free name that reads a script binding before the dump and
would read the host's or the base's global after it, and accepts a captured variable
written where it reads a host global only when it holds that global's current value
(it accepted any host name before). Found by 0174 (plan.src.js resumed into the host's
`tools.read`).

## D2604: Dump commands in the CLI: idle points numbered by answers
date: 2026-11-04 · by: repl · issue: 0174
An idle point of `djs run`/`resume` is a stop with only host calls open, before the run
answers one; the run answers one call per idle point (D2601), so idle points are numbered
from 1 the same way in every run of a script. `run --dump-after K OUT` dumps at idle point
K and stops; `resume` continues; `--dump-every-idle` dumps and reloads at each. The REPL's
`.dump` passes its waiting inputs as the dump's handles under their labels, and
`.dump-every` does the round trip at every idle point inside a run, carrying the running
input's result through the dump as the handle `$entry` (after a reload the VM's own outcome
is the load's). Host globals are `console`, `tools`, `print`, `later`; host functions are
named by `Tools::call_name`. Dumps are detected by the loader's `dump_format`, not a
prefix match. Rejected: wrapping `--body` scripts as an arrow in a Script so their top
level can be dumped; the exporter learns async-body functions instead (core fix).

## D2619: Golden format 2 dumps change only with a decision
date: 2026-11-04 · by: portable · issue: 0178
tests/dump/v2/golden/ holds dumps the exporter wrote of small programs, one per operand
role sequence, loop kind, re-enactment and class case plus a codemode body, at every
idle point (`NAME.K.dump.js`), and every gate run loads them and finishes the runs
(`golden_dumps_load_and_finish`). They stand for the dumps hosts keep (D2560): a build
that cannot load one would strand them. So, as for format 1's fixture, a golden dump is
never regenerated to make a build pass; changing or removing one needs a decision that
says why dumps of that shape may stop loading (while djs is unreleased, D024 allows an
incompatible format change, which then rewrites all of them in the same change). Adding
programs is free; their dumps are written by `write_golden_dumps` (ignored,
`DJS_GOLDEN_WRITE=1`), which never touches a program that has dumps. The exporter-only
round trip (`exporter_round_trips_at_every_idle_point`) stays the check of today's
exporter; the goldens check today's loader against yesterday's exporter. The
across-builds script (scripts/dumps-across-builds.sh) is run by hand, not by the gate:
it builds a second tree (about 30 s).
## D2630: Host-driven Sandbox runs, the first piece of the Sandbox run API
date: 2026-11-03 · by: example · issue: 0179
`sandbox.begin(code, params)` returns a `Run` (`state`, `calls`, `value`, `error`,
`settle(id, {ok, value})`, `advance()`, `dump()`) and `Sandbox.resume(dump, opts)` returns
`{ sandbox, run }`: the shape docs/continuation-dumps.md plans for 0175, of which 0179
builds what its example needs (no journals, no packaging). A host-driven run never calls
async host functions: it lists them in `run.calls` and the host settles them, so a call
can outlive the sandbox or process that made it; sync host functions are called as in
`run`. The sandbox runs nothing else while a run is in progress. The run's result is its
async body's promise, taken at the first idle point with the new export `vm_outcome`
(the entry call's outcome while no frame is active, also at IDLE; repeatable, unlike
`vm_result`) and named `run` for the dump; `resume` takes it back from the load's handle
`run` (refusing a dump without one), and DONE reads that promise instead of `vm_result`,
since a loaded VM's entry call was the dump's Script.

## D2631: The dump exports carry the host's function names both ways; changed in place
date: 2026-11-03 · by: example · issue: 0179
`vm_load_dump(vm, names_len, len)` stages the sandbox's host function names in id order
(`\n`-separated) before the dump: they give `LoadOptions::host_fn` (an open call's or a
host function's name to its id). On success out holds what the load returned: u32
`n_calls, n_handles, (call_id, fn_id)*, (handle, name_len)*`, then the handles' names;
the host frees the handles. `vm_dump`'s staging adds two lines to the host globals and
names: the same name table (tab-separated, id order; `DumpOptions::host_fn_name`) and the
values to name (`handle:name`, tab-separated; `DumpOptions::handles`). The signatures
change in place without an ABI version bump (D024): the dump exports are optional,
behind djs-wasm's `dump` feature, and only the Sandbox of the same package calls them.

## D2632: Dotted host function names make a namespace object
date: 2026-11-03 · by: example · issue: 0179
`functions`/`asyncFunctions` (and `defineFunction`/`defineAsyncFunction`) take dotted
names: `"tools.sleep"` defines `sleep` on a global object `tools`, which the sandbox makes
for its first such name and keeps a handle to, as `console.log` always was. Dumps and loads
name host functions by these full names (`$dump.pending(2, "tools.sleep", [5000])`), so a
Sandbox's dump of codemode-shaped tools resumes in the CLI (`djs resume`, whose tools are
`tools.<name>`) and the other way round. One level only (`a.b.c` is the property `b.c` of
`a`); after `fromBytes`, a new dotted name makes a new namespace object instead of
extending the restored one.

## D2620: A codemode body's frame is an async function's
date: 2026-11-04 · by: portable · issue: 0172
D2617 resumes a body as `async function (params) {...}` and refuses it where the two
maps differ. They differed in the most common pi shape: a closure over a parameter
(`tools`) makes it a boxed local, and a body declared its parameters before the async
state local `<state>`, while an async function adds `<state>` first (`start_flavor`
before the parameter list). The parser now starts a body's async flavor before
declaring its parameters, so a body's frame is laid out exactly like the function's
(`tests/smap/e2e/using_body.smap` shows the swap). No `COMPAT_VERSION` bump: a frame
saved in an image stays read by the bytecode compiled with it. Rejected: matching slots
by name in the exporter (two layouts to keep apart, for no gain).
## D2633: A dump names the run's result without a handle; `vm_outcome` removed
date: 2026-11-03 · by: example · issue: 0175
Supersedes D2630's `vm_outcome`. A run the sandbox started keeps no handle to its result
promise while it waits: `vm_dump`'s handle 0 (`0:run`) names the entry call's outcome
(`Vm::outcome`) directly, and DONE reads `vm_result` as before. Holding a handle across
idle points changed the handle table, so a run recorded with a journal no longer hashed
like its replay; now `Run.dump()` leaves the heap byte for byte as it was, and a recorded
run can be dumped and still replays. A resumed run's result is the handle its dump named
`run` (its entry call was the dump's Script).

## D2634: `run`, `runAsync` and `replay` are built on the host-driven run
date: 2026-11-03 · by: example · issue: 0175
One loop (`drive`) runs a run to its next idle point or its end, and lists new async
calls; a host-driven run (`begin`) returns there. `run`/`runAsync`/`replay` add, through
two hooks, what they did before: call each async host function as soon as the script
calls it, pass the results that came in back before every fuel slice (not only at idle
points, so a host's I/O overlaps the script's work and the inbox order is what it was),
and at an idle point wait for the next result (replay: take it from the journal). Journals
record the same entries; `begin(code, params, { journal })` records a host-driven run
(its `settle`s in order, a check at every idle point), which `replay` runs like a
`runAsync` journal. Every run, host-driven or not, makes the sandbox refuse another until
it ends (a host function that called `run` while `runAsync` waited used to start a second
call over the first).

## D2635: Both dumper packagings, chosen by the host; the image moves between modules
date: 2026-11-03 · by: example · issue: 0175
The Sandbox supports both packagings of D2564. A: `module` is built with the `dump`
feature (`scripts/build-wasm.sh --dump`, target/djs-dump.wasm). B: `module` is the shipped
module and `SandboxOptions.dumpModule` the dump module; `dump`, `load`, `Run.dump` and
`Sandbox.resume` then run in a transient sandbox of the dump module restored from the VM's
arena as it is (dumping leaves the main VM untouched, as in A), and a load collects there
and replaces the main sandbox's VM with the image (handles, such as the loaded run's, are
roots and keep their numbers). Given `dumpModule`, the Sandbox always goes through it,
which lets one module test the transport. Measured at 0175: the dumper adds 274 KB raw,
96.8 KB gzip (443.8 -> 540.6 KB gzip, +22%), and B costs 0–1 ms per dump or resume over A
(the image copies). Which one hosts ship is 0177's measurement after images are retired;
both are tested in the gate (a debug module without the feature beside the one with it,
js/test/dumpresume.test.ts).

## D2621: A class's hidden bindings in a waiting frame are dead; private names are the class's
date: 2026-11-04 · by: portable · issue: 0172
A frame that declared a class holds the class's hidden bindings (`<fields N>`, the
instance fields' initializer; `<key N>`, computed field keys; `<#m>`/`<get #x>`/`<set
#x>`, private methods' closures; decorator state) in boxes the class's closures
capture. The exporter refused them ("an internal function of a class"). The class
statement writes them and only the class's closures read them, and the dump writes the
class from its text, which makes its own; so after the statement the frame's slots are
dead and are written as `undefined` (run again in a loop, the statement writes them
again). A private name held as a value, the frame's `#x` binding or the key operand of
`this.#x += await v`, was written as `Symbol("#x")`, a different symbol (the resumed
assignment threw); it is now `$dump.private(C, "#x")`, a two-argument form of the
primitive that returns class `C`'s private name, and refused when the dump does not hold
the class. Known gap (a follow-up): a class's computed member keys are evaluated again
where the dump writes its text, which reads other bindings than the original did.

## D2622: Format 2 dumps carry no helper code; the helpers are loader primitives
date: 2026-11-04 · by: portable · issue: 0182
The owner asked why a dump starts with "all these roots": the exporter wrote JavaScript
helpers into every dump that used them, `$dump.builtin` with its table of fourteen realm
roots and a path parser, `$dump.advance`, `$dump.fromSync` (a JavaScript async-from-sync
iterator), `$dump.keep`, `$dump.bytes`, `$dump.fn`, `$dump.rejected`, `$dump.spent`.
Format 2 dumps only load through `Vm::load_dump`, which passes `$dump` in, so these are
now loader primitives (vm/dump_prims.rs) and a dump holds only its data, its functions
and `$dump.*` calls. `$dump.builtin(path)` resolves a registry path (D2371) on the
loading VM and must reach a base builtin, else it is an error; `$dump.advance(it, n)`
steps array, string, Map and Set iterators in Rust (no `length` or element getter runs;
the index is kept as an int, as `next` keeps it, which a second dump reads);
`$dump.fromSync`/`$dump.closeThrow` make the engine's own async-from-sync iterator and
its handler, so a resumed `for await` dumps again like the original; `$dump.spent(k)`
replaces `$dump.spent()[k]`. The primitives are read-only properties of `$dump`: a dump
cannot redefine them, and defines nothing there but the locations it fills. Dumps of the
corpus shrink (the goldens from 87 to 46 KB, the sleep example from 3,159 to 1,308
bytes).

## D2623: Computed keys the dump would evaluate again are refused
date: 2026-11-04 · by: portable · issue: 0182
A class's text holds its member keys, and an object literal method's text its key, so
the dump evaluates `[k]` again where it writes the text, with other bindings in scope: a
ReferenceError at load, or silently another key. The parser now records, in each
function's suspension map, the computed keys its code evaluates (`computed_keys`, spans
from `[` to `]`; `suspension_maps` keeps functions that have keys but no sites). For a
class or method it writes by text, the exporter refuses every key inside that text that
an enclosing function evaluates, unless its value cannot differ: a string or number
literal, or a well-known symbol (`Symbol.iterator`, unless a user global named `Symbol`
exists). Keys evaluated inside the written function (a literal in a method body, a
destructuring pattern in its parameters) are its own and run again when it does.
Rejected: writing the keys by value (the class text is verbatim: rewriting it changes
`toString` and the functions' texts the checker and the maps go by).

## D2624: A dump carries Math.random's state
date: 2026-11-04 · by: portable · issue: 0182
The xorshift state of `Math.random` (the realm's RandomState) was not in a format 2
dump, so a resumed run restarted the loading VM's sequence (0176: the same number before
and after a dump). Every dump now writes it first, `$dump.random(lo, hi)` (the two int32
halves as the VM keeps them), and the loader sets it; resume continues the sequence
exactly. Only the math state is set: the per-VM string hash key (D2433, derived from the
seed when a VM is seeded) stays the loading VM's. Nothing observable depends on it
(iteration orders are insertion orders; the key only spreads hash tables).

## D2625: The golden dumps are rewritten for D2622 and D2624
date: 2026-11-04 · by: portable · issue: 0182
Under D2619's rule: the golden format 2 dumps (tests/dump/v2/golden/) were written by
0178's exporter with helper code and without the random state. The format changed in
place (D024, unreleased): helper definitions are now refused by the read-only primitives,
and the new dumps lead with `$dump.random`. All 59 were rewritten by `write_golden_dumps`
from the same programs; apart from the helpers and the new first line their text is
unchanged (checked line by line). New programs: spent.src.js (`$dump.spent`), bytes.src.js
(`$dump.bytes`) and random.src.js (the sequence continues). tests/dump/v2/steps.dump.js
lost its helper definitions the same way.
## D2650: pi's future shape in the adapter: prelude-next, `next` mode, session dumps
date: 2026-11-03 · by: example · issue: 0176
tests/pi/prelude-next.ts is pi's prelude with no in-flight state: tools and globals call
the adapter's async host function `call(kind, name, argsJson)` (so an open call is
`$dump.pending`), and the body's promise, `finished`, the output counters and the store
(contents and writes) are the adapter's. The sync bridge answers `output`/`store`/`load`
with nothing, a limit's message (the same text pi's prelude throws) or the stored JSON.
Results are decoded and errors made by prelude functions (`decode`, `failure`), and the
body's outcome is encoded by `encode`/`describe`, which the adapter calls while async
calls may be pending: `vm_run` then returns IDLE, not DONE, and `vm_result` has nothing, so
their return value comes from the export `vm_outcome` (the entry call's outcome while no
frame is active, also at IDLE). D2633 removed it when the Sandbox stopped needing it; it
is back for this host, which calls VM functions between results. One visible difference: the
adapter reports `done` once the job queue has run empty, so output that a job queued
before the body settled still counts (pi posts `done` from a reaction, first). js/pi/vm.ts
runs it with `shape: "next"`; `dumpSession()` returns `{ dump, state }` (JSON: the format 2
dump with the prelude's globals as host globals and the body's promise as handle `run`,
plus the adapter's state), `resumeFromDump` reruns the host globals and the prelude (or
restores the prelude image), loads the dump and restores the state, and `openCalls()`
lists what waits, for the host's policy (deliver a recorded result, issue again, reject).
pi's own prelude and mode are unchanged.

## D2651: `dumpEveryIdle`: the next mode's stress mode
date: 2026-11-03 · by: example · issue: 0176
With `dumpEveryIdle`, a next-mode session dumps itself at every idle point where a call is
open and continues in a new instance of the module resumed from the dump, invisibly to
pi's host. pi's suite (`node js/pi/pi-tests.ts --next --dump-every-idle`) and every
pi-vm.test.ts test run that way, as `--dump-every-idle` does for the difftest corpus.
Every merge's gate runs pi's suite in the next mode on its release module (tests/pi/
expected-failures.txt); the dump-every-idle run needs a release module with the dumper
(one more LTO build), so it is `scripts/pi-dump-every-idle.sh`, for the gate's full run.

## D2652: prelude-next has no lockdown; the next mode has frozen builtins
date: 2026-11-03 · by: example · issue: 0176
pi's lockdown replaces builtin data properties by accessors whose functions live in the
prelude: builtin state a dump cannot write (the exporter refuses them) and the rerun
prelude could not recreate in place. Frozen builtins (D2304) do the same job inside the
VM, are part of its creation options, and leave no patches to write. pi's suite fails the
same 11 tests with pi's prelude, with it minus the lockdown on frozen builtins, and with
prelude-next. The adapter's own `performance` global is now frozen in every mode, as the
lockdown left it.

## D2653: The pi adapter counts fuel across sync host calls
date: 2026-11-03 · by: example · issue: 0176
A `vm_run` that parks on a sync host call returned before its fuel ran out, and the next
slice started with full fuel, so a loop of sync host calls (`while (true)
tools.echo(1)` in pi's mode, `while (true) load(k)` in the next mode) never reached an
interrupt poll and hung an in-thread VM. The new export `vm_fuel_used` (the last
`vm_run`'s fuel, from `Vm::run_metered`) lets the adapter carry the fuel left across host
calls and poll once a poll's worth is used. Scripts without sync host calls poll exactly
as before; others poll at other points, so DETERMINISTIC_VERSION is 3.

## D2700: Heap images are no longer a persistence format; what was removed
date: 2026-11-04 · by: example · issue: 0177
D2560 made continuation dumps the way a waiting run persists; 0177 removed what only
served images as persistence. Superseded and removed: the snapshot container (D2260, the
`DJSN` format and its checks, D2400), host-side compression against a reference image
(D2261; reference images as such, D2262's role as a snapshot reference, though pi's
prelude image stays and stays deterministic), session images (D2066), `Sandbox.bytes()`
and `Sandbox.fromBytes()` (D2263), pi's `snapshot()` and `resumeVm()` (D1890's snapshot
part), the REPL's `.snapshot` and restoring images with `.restore` (D2340's containers),
`djs heap` reading containers and session images, its `--reference` option and
scripts/heap/reference.ts (D2264's container part), djs-cli's zstd dependency, and the
research scripts that measured containers. Kept: the base image (D1321), pi's prelude
image cache (D1495), `Vm::image`/`Vm::from_bytes` for the stress and determinism checks
and the docs examples, `djs heap` on raw images (the REPL's new `.image` writes one), and
images as the transport to the dump module (D2635). Verification (D1056): `Vm::from_bytes`
still verifies every cell, for the tools and stress modes, which use it as a heap check;
djs-wasm's `vm_from_bytes` now loads with `Vm::from_bytes_trusted` (header and dependency
descriptor only), since every image it loads was made in the same process, so the
shipped module no longer carries the verifier (1,038,666 -> 1,029,026 bytes, 444.0 ->
440.5 KB gzip). The dependency descriptor (D2265) stays: it guards the transport between
two modules and the stress modes.

## D2701: The dump module's instance and VM are kept between dumps
date: 2026-11-04 · by: example · issue: 0177
With `dumpModule`, a Sandbox made a fresh dump-module sandbox per dump (D2635), paying an
instance and whatever the dumper derives per VM (its builtin name table, D2626) every
time. It now keeps one dump-module sandbox for its lifetime and loads each image into the
same VM with the new export `vm_replace_image` (`Vm::replace_image`: the image replaces
the VM's heap, the VM's own state outside the heap stays). Measured at 0177 on the
harvested corpus, packaging B's dump and resume p50 are 2.8 and 0.9 ms, level with
packaging A (0184's run with a fresh instance per dump measured 3.5 and 1.3 ms, at
another machine load); D2626, once on main, makes later dumps of a kept VM cheaper still.

## D2702: The Sandbox's `maxFuel` counts fuel across sync host calls
date: 2026-11-04 · by: example · issue: 0177
As in pi's adapter (D2653): `Sandbox.drive` adds `vm_fuel_used` after every `vm_run`, not
a whole slice at OUT_OF_FUEL only, and checks the budget at host calls too. A loop of
sync host calls (`for (;;) f()`) used to restart every slice with full fuel and never ran
out; it now fails with `fuel` like any other loop.

## D2703: Hosts store dump text compressed with zstd; the format stays a plain Script
date: 2026-11-04 · by: example · issue: 0177
A format 2 dump is not wrapped in a container: hosts compress the text when they store it,
with zstd at level 3 (node's `zlib.zstdCompressSync`), and decompress it before
`Sandbox.resume`. Measured (0184's dump-costs, the harvested corpus's 1,063 idle points):
6.1 KB raw and 1.7 KB compressed at the median, 17 KB at p99; a third of the image
containers' size (5.1 KB median), with no dictionary or reference image to keep, and
readable by any zstd and any later djs. Level 19 saves a fifth to a quarter more on
megabyte dumps (big-log 394 -> 299 KB) at a much slower compression; the host may choose
it for archives. pi's `{ dump, state }` is JSON and compresses the same way. Rejected: a
djs container with a header and compression inside the engine or package (a second
format to version, and a codec in the module).

## D2704: Packaging B: the shipped module has no dumper, hosts that persist load the dump module
date: 2026-11-04 · by: example · issue: 0177
Confirms D2564's lean. Measured after 0177's removal (`wasm` profile): the module without
the dumper is 1,029,026 bytes, 440.5 KB gzip; with it (`djs-dump.wasm`) 1,332,899 bytes,
548.9 KB gzip, +108 KB gzip (+24.6%), which A would add to every host, also the ones that
never persist a run (pi's in-thread and worker launchers start a module per execution).
With the dump-module VM kept between dumps (D2701), B's dump and resume times are level
with A's (corpus p50 2.8 / 0.9 ms; cases within noise), so B costs only a second module
to ship and an image copy per dump. The size ceiling stays on the main module and was
lowered after the removal (444,500 -> 441,522; slim 379,900 -> 376,679). Hosts get the
dump module from `scripts/build-wasm.sh --dump`; a module built with the `dump` feature
still works alone as `module` (A) where one file matters more than size.
## D2201: The guide's site follows Elanor d3184572: its retuned type scale
date: 2026-11-03 · by: design · issue: 0185
The vendored Elanor (D2200) moves from ccdb0d9e to d3184572 (`scripts/elanor-update.sh
main`). What changed upstream is typography only. The font files are the same (Mallos
Serif 0.1.1 was already vendored); the tokens are now tuned to it, because it draws its
letters at 1.1 times the earlier font's size:
- every text style is a step smaller (prose 18 → 16 px, UI 16/14/12 → 14/12/10 px, the
  UI label 16 → 14 px), and the small UI styles are heavier (secondary 500, tertiary 550);
- headings are lighter (app headings 500 to 600 by size), the selected item is 550;
- primitive font weights are numbers (`--primitive-font-weight-700`, with half steps 450
  and 550) instead of names;
- inline code is sized by a new token, `--semantic-number-font-scale-inline-code`
  (0.9375 of the surrounding text).
Colors, font families, the code style and the Shiki theme did not change, so the
highlighter roles and the diagrams stay as they are.
The site takes the new tokens as they are, with these choices of its own: inline code uses
the scale token everywhere (the per-heading and per-table overrides are gone, since the
code follows its surroundings); tables, search results and the live widget's buttons use
the UI's primary style (14 px; the secondary style is now 12 px, too small for dense
reference tables and controls); the footer uses the secondary style instead of the 10 px
tertiary one. The token package now ships an ISC license, vendored as
`assets/elanor/tokens.LICENSE` and linked from the footer.
Elanor's preview builds still expire after 30 days, but a maintainer can now keep a main
build permanently as `main-<12 hex>`, and releases get permanent `vX.Y.Z` labels. Nothing
is on public npm yet, so the vendored copy stays.

## D2626: A dump's builtin names are walked once per VM, and walked faster
date: 2026-11-04 · by: portable · issue: 0184
Every `Vm::dump` named the base's builtins by walking the realm (`registry::reachable_names`):
0.8 ms of a 0.8 ms small dump natively, about 2-3 ms in wasm (0184's measurements). The
walk now skips edges to cells it has named already before making their paths (the same
names, half the time), and the VM keeps the table after its first dump (with the base's
length; another base is walked again): a VM's next dumps cost 0.05 ms natively for an
empty session. Packaging B makes a fresh dump-module VM per dump, so it gets only the
faster walk unless the host keeps that instance between dumps.

## D2627: Subroutine return addresses in dumps are gosub indexes; case tests keep their discriminant
date: 2026-11-04 · by: portable · issue: 0184
A finally block, and a for-await's close subroutine entered by `break`/`continue`/`return`
through the loop, run as subroutines: `gosub` pushes the pc after it, and in the block the
stack holds the value the block carries (`undefined`, the exception or the return value)
and that return address. Suspension maps describe them as roles `finally.value` and
`finally.return`. A pc is this build's, so a dump writes the return address as the index
of its `gosub` among those that enter the subroutine, in emission order (the map's
`Site::returns` lists the pcs; the loader maps the index back and refuses one it does not
have). Emission order follows the source (jumps out of the try block as they appear, the
normal exits, the exception path last); a compiler that orders them differently bumps
`COMPAT_VERSION`. The close subroutine of a for-await no longer keeps the iterator under
its await (`v ret it` became `v ret`, closing consumes it): its stack is the two roles
only. A `switch`'s case test keeps the discriminant twice (`dup`, then compared): role
`switch.discriminant`, written twice. The thunks of `Promise.prototype.finally` and the
handler AsyncIteratorClose of a throw puts on `return()`'s promise (`PromiseValueThunk`,
`PromiseThrower`) are written as arrows (`() => (v)`, `() => { throw v; }`) where they
react in flight. Still refused: a finally's saved completion value (Script and eval code,
which do not suspend) and closing a for-await iterator on the return path of a resumed
generator (`return(v)` at a yield).

## D2628: Dumps write using Records, destructuring, for-in and builtin iterators from their state
date: 2026-11-04 · by: portable · issue: 0184
The constructs suspension maps still called unsupported get roles, and the builtin
state they keep gets a loader primitive that makes it from data, checked where the
builtins trust its shape:
- `using`: the resource Record stays under the statements (`using.record`; the
  disposal's `s v ret s` adds `finally.value`/`finally.return`, D2627). Written once,
  shared by both stack slots: `$dump.using([value, method, kind, …], { error, hasError,
  needsAwait, hasAwaited, next, value, pending, hasPending })`, the state by name (only
  slots that hold something), not by builtins/disposable.ts's slot numbers.
- Destructuring: an object pattern keeps its value (`pattern.source`), the keys a rest
  property excludes (`pattern.excluded`) and a computed key (`pattern.key`); an array
  pattern `next` and the iterator (`pattern.next`, `pattern.iterator`, `undefined` once
  done); targets their references (the assignment roles); an assignment pattern
  expression its value (`assign.value`). Plain operands, no primitive.
- for-in: the key iterator is an internal array; the dump writes `$dump.forIn(o, keys)`
  with the keys not yet visited (deleted ones are still skipped at each step, as in the
  original), in `loops` as `{ kind: "for-in", iterator }`. A for-in over a Proxy (a
  Record its traps fill) is refused.
- `matchAll`'s iterator: `$dump.regexpIterator(re, s, flags)` over the regexp clone it
  holds (written with its `lastIndex`); iterator helpers: `$dump.iteratorHelper(slots)`,
  builtins/iterator.ts's slots as they are (kind and state checked; a wrong slot makes a
  step fail with an internal error, never more). Both refuse another prototype.
Still refused: a site inside a class definition (the class under construction is on the
stack; the dump makes classes from their text) and closing a for-await iterator on the
return path of a resumed generator. Also fixed on the way: an array with named
properties that a frame holds once was written inline, its patches without a location.
## D2660: The gate is legs and builds in scripts/gate.py
date: 2026-11-04 · by: gate-speed · issue: 0183
The serial gate.sh (239–573 s, median ~400 s, every merge) became a table of jobs in
scripts/gate.py: legs (checks, each with an input class and the builds it needs) and builds
(cargo invocations whose outputs the legs run); gate.sh only execs it. A table makes the
schedule, the cache and the full run one mechanism instead of shell plumbing per step. New
checks are added as legs. Supersedes D1792's "the gate is gate.sh's steps in order"; the
persistent gate worktree stays.

## D2661: Every build configuration builds at once in its own target directory
date: 2026-11-04 · by: gate-speed · issue: 0183
Cargo locks a target directory's profile while it builds, so the serial gate's cargo
commands are kept as they were (the same packages, features and profiles, so the same
binaries) but each configuration gets a directory of its own and they build concurrently.
Legs run the built binaries instead of `cargo run` (hard links in target/gate/bin, which a
later build cannot change), and `cargo test --no-run`'s test binaries run in parallel the way
`cargo test` runs them one by one (package directory, CARGO_MANIFEST_DIR,
scripts/test-runner.sh). The first failure stops the rest. Unifying the configurations
(one djs-core build for test262, difftest and the docs) would save CPU but changes what is
tested, so it was not done.

## D2662: Gate legs are cached by input hash
date: 2026-11-04 · by: gate-speed · issue: 0183
A leg's key hashes its name, the committed tree restricted to its input class (D2663) and
what it reads outside the repository (toolchain, qjs binary, vendor/test262 and
vendor/unicode; pi-private for pi's legs). A passed key is a file in
work/scratch/gate-cache/ of the primary checkout; a leg whose key is there is skipped, and a
build runs only if a leg that needs it runs. Written only from a clean tree; missing,
unreadable or corrupt entries mean "run"; 14 days unused are pruned; `--clear-cache` and
`--no-cache` exist. Sound because a leg's verdict is a function of exactly those inputs: the
same inputs passed before. Legs that read more than the tree (`registry` compares with main)
or cost nothing (`fmt`, `paths`) always run. A docs-only merge gates in ~8 s instead of
~250 s, a js-only one in ~12 s.

## D2663: Three input classes, guarded
date: 2026-11-04 · by: gate-speed · issue: 0183
`all` (the guide and djs-docs's tests: djs-docs reads the whole repository), `nodocs` (all
minus docs/, work/, .github/, the top-level *.md: the node legs and pi's tests) and `rust`
(nodocs minus js/ but js/src/layout.generated.ts, and minus examples/: everything else).
Defined by exclusion, so unknown paths are inputs of every leg; scripts/ is in every class,
so changing the gate reruns it all. Per-crate selection (djs-docs changes not rerunning
djs-core's tests) was left out: little gain, more ways to be wrong. The always-run `paths`
leg fails when Rust code outside djs-docs names a path outside `rust` in a string literal
(PATHS_ALLOWED lists the reasoned exceptions); paths built at run time are left to the full
run.

## D2664: `--full` and full-only legs; a full run daily
date: 2026-11-04 · by: gate-speed · issue: 0183
`scripts/gate.sh --full` reads no cache and adds the full-only legs, checks too slow for
every merge (the first: pi's suite with a dump at every idle point, D2651, on its own LTO
module). It must run on main at least daily (the coordinator) and before a release, not
before every push; passes go to work/scratch/gate-cache/full-passes.tsv and every other gate
run prints a reminder once the last is more than 24 hours old. It catches what the cache
cannot: flaky tests and reads outside a leg's class that the `paths` leg does not see.

## D2665: The gate runs new executables once while the build goes on
date: 2026-11-04 · by: gate-speed · issue: 0183
macOS assesses every new executable on its first exec (syspolicyd and XProtect, serialized,
~0.1–2 s each; syspolicyd at ~250% CPU during a gate). Tiny test binaries waited 30–60 s,
so the test leg took 84–125 s for ~15 s of work. Builds now pipe cargo's JSON through
`gate.py stream`, which runs each new test binary with `--list` (and the `djs` and
`djs-difftest` programs with their harmless usage) as soon as it is linked, and legs run hard
links rather than copies (a copy is a new file, assessed again). The test leg went to ~13 s;
the scans still cost build time. Adding the app that runs the agents to System Settings ›
Privacy & Security › Developer Tools skips the assessment; that is the owner's call.

## D2667: difftest runs stress variants as separate tasks, longest first
date: 2026-11-04 · by: gate-speed · issue: 0183
A case and its stress variants ran as one task, so a slow case serialized its five runs on
one worker; now each run is a task, compared with the plain run after all finish, and
`--times FILE` orders tasks by the last run's times (the gate keeps one file per leg).
builtins/array-from-arraylike.js built a 100000-element array, quadratic under GC stress
(a collection per callback copies the growing array: 77 s, the whole difftest leg's wall);
it builds 10000 now, through the same %EachDense path (mode 8, D2306, no length threshold),
still under GC stress. That quadratic cost is what GC stress does, not an engine bug.
vm::tests::hot_loop_is_invisible runs its 16 traces on threads (33 s → ~9 s).
## D2269: No `djs heap` command and no `.image`; the REPL's `.heap` keeps the heap report
date: 2026-11-04 · by: snapshot · issue: 0187
Owner: "Remove `.image` and the `heap` command (but not `.heap`). We no longer need that."
Since D2700 images are an in-process mechanism, so writing them to files (`.image`) and
analyzing files (`djs heap <image>`, with `--diff FILE`, `--only`, `--all`, `--json`) had
no remaining use. The analyzer (crates/djs-cli/src/heap/: graph, structural diff,
sections, D2264, D2432) stays behind `.heap [sections]`, over the live VM against the base
image; what only the command used goes: file reading, the options, the summary's file and
dependency-descriptor lines (the live VM is always this build's), the whole-image `--all`
view (the analyzer still describes the whole image when given no reference). The sections
keep their JSON form, which the tests check; nothing prints it. Supersedes D2264's and
D2340's command and `.image` parts.

## D2666: The gate runs outside the merge lock; only the fast-forward is inside
date: 2026-11-04 · by: gate-speed · issue: 0183
merge-to-main rebases and regenerates without a lock, gates under a gate lock (guarding the
one gate worktree), then takes the merge lock only to fast-forward main, and only if main is
still the commit it rebased onto. If main moved, it rebases and gates again; the cache
(D2662) makes that cost what the move can affect (a coordinator's docs commit: the guide
leg). The third attempt holds the merge lock from rebase to fast-forward, so a branch
cannot starve. Locks are always taken gate lock first, so merges never wait on each other in
a cycle. The guarantee is D1792's: main only ever fast-forwards to a commit that itself
passed the gate. The coordinator's commits (with-merge-lock.sh) wait for a fast-forward,
not for a gate. Older merge-to-main copies (branches not rebased since) gate under the
merge lock alone; the new one waits for the merge lock once before taking the gate
worktree, and gate.py refuses to count a run whose worktree another checkout moved
(flock per worktree, HEAD checked before caching and at the end), so a mixed pair fails
safely. One gate worktree still serializes gates; a second is the next lever if the gate
lock becomes the queue.

## D2668: Clippy on every feature combination that ships
date: 2026-11-04 · by: gate-speed · issue: 0183
Clippy ran on the default features only, so a warning in the slim build with the dumper
(`--no-default-features --features dump`) reached main unnoticed. Two cheap legs now run
clippy, with `-D warnings`, on djs-core's library natively for slim, slim with `dump` and
full without `dump`, and on djs-wasm in wasm32 release for the main module (no dumper),
slim, dump and slim with dump. Libraries only: the tests build in the default combination,
and the slim tests in their own legs.

## D2669: Full runs in their own worktree
date: 2026-11-04 · by: gate-speed · issue: 0183
`scripts/gate.sh --full` in the primary checkout failed: merges fast-forward main there,
which moved the tree under the run (gate.py rejects that, D2666). Started in the primary
checkout, `--full` now checks out main detached in `.worktrees/full` (created on first use,
its target/ stays warm) and runs there.

## D2720: Suspended frames become named-slot state machines in the compiler (SM)
date: 2026-11-04 · by: owner · issue: 0188
Accepted from docs/suspension-design-review.md: at every await/yield the compiler moves
everything in flight into named frame slots, so a suspended frame is {fn text, at, slots};
no generated code in dumps (variant R rejected: dumps of resumed continuations grow per
generation); no source-level lowering of async code (variant A rejected: 2.4-5.4x slower).
Migration in the report's eight steps (issues 0189-0196).

## D2721: Slot names are construct@span; states keep kind#N
date: 2026-11-04 · by: owner · issue: 0188
A spilled slot is named by its construct and the source span of the value it holds,
relative to the function's start (`call.this@41`, `iterator@312`, `completion.kind@S`);
states keep their `await#N` names. The pins list the named slots per state.

## D2722: Open host calls carry host JSON state; save/resume hooks; the Sandbox keeps the bookkeeping
date: 2026-11-04 · by: owner · issue: 0188
`$dump.pending(id, name, args, state)`; a host function may be declared as
`{call, save, resume}`; the Sandbox tracks open calls, calls `save` at dump time, `resume`
(or `call` again, or rejects) at load, and drives the run. Functions without hooks keep
today's behaviour.

## D2730: Slot names: offsets from the function's start of the element or the construct; pins list them
date: 2026-11-04 · by: sm · issue: 0190
Makes D2721 exact (docs/continuation-dumps.md "Slot names"). A temporary's name is its
construct (today's operand role without the index) and a byte offset from the function's
start: a list element's own start (arguments, array elements, template parts), which
replaces the index, else the construct's start (the callee for `call.callee`/`call.this`,
the target for `assign.*`, the binary expression, the `for`, the `switch`, the pattern's
`[`/`{`, the `finally` keyword, the first bound name of a `using` region). Start
offsets, not start..end spans: the start alone is unique per site once the construct
name is part of the key, and ends add nothing a later compiler could keep stabler. Copies
of one value (a `case` test's discriminant, an `await using` disposal's values) are
named by the `case` and by the site until 0192/0193 hold them once. The suspension maps
report the names per site (`Site::slots`), and the pins (tests/smap) list them in place
of the role sequence, which is the role order bottom first; summary.txt still counts
role sequences. Pins rewritten under this decision (D2619); no dump or behaviour
change.
## D2770: A call's state is JSON text, written verbatim as `$dump.pending`'s fourth argument
date: 2026-11-04 · by: hostcalls · issue: 0189
The engine keeps no host state: the host passes each open call's state as JSON text when
it dumps (`DumpOptions::call_states`; `vm_dump`'s fifth staging line `id:bytes` and the
texts after it), the dumper writes it as is (`$dump.pending(2, "tools.sleep", [5000],
{"due":1793776092499})`) and the loader stringifies the value back (`OpenCall::state`;
`vm_load_dump` gives a byte length per call and the texts after the handles' names; ABI
version 3). Readable in the dump, and no wire format in the format. The dumper checks the
text first (vm/dump_state.rs): JSON only, so the host's text cannot inject code, and no
key `__proto__`, which an object literal reads as its prototype where `JSON.parse` makes a
property. A state for a call that is not open is refused too. Rejected: the state as a
heap value the dumper writes like data (needs a mutable VM or a parse at dump time, and
the host's state would get `$dump.nN` names), and a JSON string literal (unreadable).

## D2771: Tasks: the Sandbox drives the run and the open calls' hooks
date: 2026-11-04 · by: hostcalls · issue: 0189
`sandbox.start(code, params)` returns a `Task`: the sandbox calls the async host
functions as `runAsync` does and `task.result` is the outcome. `task.suspend()` waits for
the next idle point with nothing left to deliver, takes each open call's state (its
function's `save(call)`, default `call.state`), dumps and stops: the calls' `signal`s
abort, late results are dropped, `result` rejects with kind `suspended` (marked handled,
so an unawaited result is no unhandled rejection). A failed dump rejects only the
`suspend` and the task goes on; `checkpoint()` dumps without stopping. `Sandbox.resume(dump,
opts)` (and `sandbox.resume(dump)`) loads and drives: each open call goes to its
function's `resume(state, call)`, or to `call` again when there is no `resume`, and a
plain function's call is rejected, because the sandbox cannot know whether calling it
again is safe. The hooks get a `HostCall` `{id, fn, args, startedAt, state, signal}`,
also `this` of `call`, so a call can record its state when it starts and a resumed call
keeps its state for the next dump. The host-driven API stays: `begin`/`settle`/`advance`,
`Run.dump(states)`, and `sandbox.loadRun(dump)` (what `Sandbox.resume` returned before),
whose calls carry their loaded `state`.

## D2731: Slots name every frame index; the operand stack stays in memory (refines D2720)
date: 2026-11-04 · by: sm · issue: 0191
D2720 said the compiler moves everything in flight into named frame slots before each
await/yield. A suspended frame is already one array (callee, `this`, arguments, header,
locals, then the k values the code keeps past the locals at that site), and k is fixed per
site in a build (the stack analysis checks it). So the k temporaries are frame slots as
static as locals, and the map names every index: `Site::named_slots` gives each slot's
name (binding `name`/`name@scope`, hidden local, temporary `construct@offset`, D2730) and
place (argument, local, or temporary k). The dump writes one object, `slots: {name:
value}`, in place of `bindings`, `operands` and `loops`; `$dump.resume` writes each value
into the index its own map gives the name and refuses an unknown, missing or extra
name, so nothing in a dump depends on key order, push order or stack depth, and a build
with another slot set at a state refuses instead of misplacing. Binding keys are exact:
`name@scope` only when two of the name are in scope, `name` otherwise (the loader used to
take either). Temporaries never collide with bindings: every construct has a `.` or `-`
(the role `exception` is named `for-await.exception`).

What stays: the operand stack still exists in memory at a suspension. That is fine
because the format never sees it: no stack order or depth reaches the dump, and the
loader does not rebuild one, it fills indexes. A physical spill (moving the k values into
reserved hidden locals before the await and back after) would change no dump, would cost
2k moves per suspension and max-k frame slots, and would need interpreter support: a
rejected await throws at the resume pc before any unspill instruction runs, so handlers
whose depth includes statement-level values (a loop's record, a finally's value) would
find them missing; await/yield would have to carry the spill and the resume path move the
values back. It would buy a frame whose stack part is empty while suspended (a simpler
layout for a future register allocator or a frame format without a stack part) and could
be revisited then. Statement-level state (loop records, switch discriminants, `using`
records, destructuring state, finally completions) does move into real hidden locals in
0192/0193: that removes `gosub` index mapping and the special cases, not just names.

No change to bytecode, frames or images: COMPAT_VERSION stays. Pins (the `exception`
rename) and every golden and hand-written format 2 dump are rewritten under this decision
(D2619, D2642): tests/dump/v2/golden/*.dump.js regenerated by the exporter, the
hand-written fixtures converted by hand (loops.dump.js lists its slots out of frame
order on purpose).
## D2772: The pi adapter's open calls carry their pi ids as call states
date: 2026-11-04 · by: hostcalls · issue: 0195
In the next mode the adapter's call table (pi's call id per engine call) is not adapter
state any more: `dumpSession()` gives each open `call` its pi id as its state
(`$dump.pending(4, "call", ["call", "slow", "1"], {"id":2})`, D2770) and
`resumeFromDump` rebuilds the table from the loaded calls' states (a call without one
refuses the resume). `SessionDump.state.next` keeps the counters, the store and the
clock, not the calls. pi's host stays the policy for open calls after a resume (it runs
the tools; the adapter cannot), so the adapter has no `resume` hook of its own and
prelude-next needed no change: its tool calls were engine async host calls already.

## D2790: No doctests, no empty test binaries, no bgen program in the gate; guarded
date: 2026-11-04 · by: gate-speed · issue: 0197
Every freshly linked executable waits for macOS's first-run check (D2665), and the gate's
`target/` jobs ran one after another: build:tests, then a `cargo run` of djs-bgen, then
`cargo test --doc` (rustdoc builds a merged doctest binary per crate, for 3 doctests, all
`ignore`d). In 0191's gate that chain was the whole 639 s (316 + 95 + 228). Now every library
has `doctest = false`, the five programs without unit tests have `test = false`, and the
bgen leg is gone: djs-bgen's `builtins_gen_is_fresh` test, which the `test` leg runs, now
compares all four files `bgen.sh --check` compares (prelude-slim.js was missing). The
always-run `targets` leg fails when a library with `doctest = false` gains a doctest (a
Rust code block rustdoc would compile, not `ignore`d) or a `test = false` program gains a
#[test], cfg(test) or a module, so nothing that cargo would test is skipped silently.

## D2791: One integration-test binary per package
date: 2026-11-04 · by: gate-speed · issue: 0197
Each integration-test file was a binary linked against the whole engine and checked by
macOS on its first run. djs-bgen's three, djs-cli's two, djs-test262's four (0183) and
djs-core's slices, regexp_fuel and verify_fuzz are now `tests/it/main.rs` with one module
per former file (`cargo test -p X --test it [module::]`). djs-core's e2e stays its own
binary: agents run it alone (`--test e2e`, DJS_E2E_MODE). The unsafe-heap leg runs `it`
and e2e, so slices and regexp_fuel now also run in that build. Moving integration tests
into unit tests would save more but tests the crate built with cfg(test), so it was not
done.

## D2792: Builds end when cargo does; first runs overlap the legs
date: 2026-11-04 · by: gate-speed · issue: 0197
D2665's builds waited until every new test binary had had its first run. They now end when
cargo does, and the runs still waiting for macOS's check continue beside the legs that use
the binaries. Measured on the same core change (lab, loaded machine): 77 and 85 s this way,
88 and 100 s waiting, 105 and 105 s with no early runs at all. The four wasm clippy
combinations (D2668) run as four legs in target directories of their own (124 s one after
another, ~45 s side by side).
## D2732: Construct temporaries: statement state in named locals, listed where live
date: 2026-11-04 · by: sm · issue: 0192
The state a construct keeps while its body runs moves from the operand stack into
locals of its own (`VarKind::Temp`, compiler/temps.rs): a for-of/for-in/for-await loop's
iterator and `next`, a `yield*`'s, a switch's discriminant, a `using` list's resource
Record, an array pattern's iterator and `next`, an object pattern's value and excluded
keys. Their slot names are the D2721/D2730 names (`for-of.iterator@18`), so dumps of loops
keep their keys; their bindings have internal names `<tN>` (numbered per function, so a
compile adds no atoms). A temporary is a slot of a state only where it is live: a
backward liveness pass over the function's final phase-1 code (jumps, branches,
exception edges from instructions that can throw inside a handler's region, `gosub`
into its subroutine and each `ret` to the instructions after the `gosub`s to it) decides,
so code order that differs from source order (a for-of's target runs after its right
side) cannot make a map wrong, and a stale value from an earlier run of a loop is never
dumped. The pass is conservative where the code is (an `async_return` may throw into a
loop's close handler, so a for-await's iterator is listed at its close subroutine's
await). Deletions: the `loops` handling in the exporter and loader (loop records are
ordinary named slots), the doubled switch discriminant (the case test's copy is the left
side of its `===`, `binary.left@C`), the Record's copy in an `await using` disposal, and
the roles `LoopIterator`, `LoopNext`, `SwitchDiscriminant`, `UsingRecord`,
`PatternSource`, `PatternExcluded`, `PatternNext`, `PatternIterator`.

A for-of step read `it next` with `dup2` (two dispatches); from locals it would be three.
A new op `iter_next_loc loc, target` reads the iterator from local `loc` and `next` from
`loc + 1` (the two temporaries are declared together) and is emitted for `get it; get
next; iter_next`: one dispatch. It is the one jump with an immediate before its target
(`CodeBuf::finish` keeps the prefix; only plain jumps are shortened). A for-in key
iterator's `for_in_next` keeps `it key done`, so the assignment starts with `nip`.
Generators' return path marks a closed pattern or loop iterator `undefined` before
calling `return()`, so the construct's own close handler, which covers that code, does
not close it again when `return()` throws (13 test262 dstr tests showed it).

Measured (instructions retired, native release; wasm in node, n=4), base 36a7f9ed:
for-of loop -7.0% native, -3.6% wasm; forEach/for-of -3.6%/-1.5%; destructuring loop
-7.9%/-7.3%; switch -2.0%/-0.5%; await-heavy for-of -1.5%/-0.6%; the other engine
workloads within -3%..+0.5% (props +0.5%/-0.5%). In the hot loop `iter_next_loc` has an
arm of its own: one arm shared with `iter_next` saved 60 bytes gzip but cost wasm
0.5-1.2% on unrelated workloads (D1352). Size: +1.92 KB raw, +736 B gzip of the shipped
module (the temporaries' compiler code, the op in three tiers, `CodeBuf::finish`), within
the ratchet ceiling after making `temp` infallible, naming temporaries by integer atoms
and sharing one copy of the step in `step_once`.
The op table changes, so images get a new descriptor; no COMPAT_VERSION bump. 14 of 130
golden dumps refused (their states' slot sets changed: a pattern's source no longer live,
a `case` copy, a Record copy) and are rewritten under this decision; the other 116 load
unchanged, including those whose slot order changed. Pins rewritten (temporaries listed
per state, summary counts constructs).

## D2733: Finally completions in temporaries; gosub/ret and return-address indexes gone
date: 2026-11-04 · by: sm · issue: 0193
A finally block, a `using` list's disposal and a for-of's close on a jump out are code of
their own entered with a jump, after the completion is stored in two construct
temporaries (D2732) named by the statement: `completion.kind@S` and
`completion.value@S` (S: the `try` keyword, the `for`, the list's first bound name).
The kind is an int: 0 normal, 1 throw, 2 return, `3 + 2·o` a `break` to the statement
at offset o from the function's start (a loop's or switch's keyword, a labeled
statement's first label), `4 + 2·o` a `continue` to it, so a dump names the pending
exit by its target, not by an emission index. The block's end dispatches: `get kind;
jump_if_false L_end` for a normal completion, a compare per exit that entered the block
(none for the last when nothing else can arrive), and the rethrow of the value (cleared
first). An exit that crosses several blocks enters the next one out from the dispatch
of the one before, with the same kind; a return copies its value along. Compiled code
and compiler: compiler/completion.rs; docs/compiler.md 7.4.

All functions use it, not only those that can suspend (the review's option): one lowering
keeps the compiler simple and lets gosub/ret go entirely. Deleted: the `gosub`/`ret` ops,
the `GOSUB` flag and the stack checker's and liveness pass's subroutine handling, the
roles `finally.value`/`finally.return`, `Recorder::subs`, `Site::returns` and the
exporter's and loader's return-address index mapping (D2627's mechanism; its case-test
part stands), `OpenIter::ForOf` (a generator's return from a for-of head's target now
leaves the loop through its close block: the env is pushed while the target is stored),
the inline for-await close on a generator's return path and with it the refusals
"closing a for-await's iterator on a generator's return(v) path" and "a finally's saved
completion value" (Script and eval code save it in `completion.saved@S`). Statements
keep nothing on the operand stack any more, so `stmt_depth`, `BlockEnv::depth` and the
stack cuts before jumps and returns are gone too.

Liveness: only completions with a value write it, and the kind is written after the
value, so a write of `completion.kind@S` defines `completion.value@S` for the liveness
pass. A value is then listed only inside its block (where the dispatch may read it); a
normal completion there shows `undefined` (the rethrow clears it).

Measured (instructions retired, native release; wasm in node n=4), base f7bd3f3d: a
try/finally, return-through-finally, for-of exit loop -4.5% native, -3.8% wasm; the
engine workloads within -1.7%..+0.5% native and -0.1%..+0.6% wasm. Size: -156 B gzip
(main), -113 B (slim). Frames of functions with for-of loops get two more locals
(slots of disjoint scopes are shared). Pins and snapshots rewritten (the new slots, pcs);
19 of 130 golden dumps (finally, for-await-close, using: states inside a finally block,
a disposal or a close) are rewritten under this decision, the other 111 load unchanged; a
new golden program (return-path, 2 dumps) dumps the formerly refused for-await close. The op table
changes, so images get a new descriptor.

## D2734: No role vocabulary: stack values are named by construct; class sites refused by a marker
date: 2026-11-04 · by: sm · issue: 0194
The `Role` enum (with list indexes, `call.arg[2]`) and `Site::operands` are gone. The
parser pushes the constructs of the values an expression keeps on the operand stack
(`names_push(at, &["call.callee", "call.this"])`) with the offsets that name their slots
(D2721, D2730); a site keeps their names (`Site::stack`, bottom first) and
`NamedSlot::holds` is `Temp` without a role. Nothing read a role beyond its name: the
exporter and loader already went by slot names (D2731). The depth check stays as an
internal consistency check: a site whose stack holds a value the parser did not name is
unsupported ("N operands, M named"), never misnamed, since the loader sizes the frame by
the names. Class definitions keep refusing their suspensions (heritage, computed keys,
decorators), now by an `unsupported_begin("class")`/`unsupported_end` marker instead of a
pseudo-role: the half-made class is not a value a dump can write (classes are written by
their text, which defines them whole), so this is the one refusal of the old
`unsupported:<construct>` set that stays. The last statement-level stack value, the
exception a `for await` rethrows while its iterator's close is awaited, moves into a
temporary `for-await.exception@S` (a local; same name as before), so statements keep
nothing on the operand stack at any site. Slot names at every site are unchanged; the pins
change only in local numbering (one more local per for-await loop), the summary in its
construct lists (no indexes), and no golden dump changes. No image format change:
COMPAT_VERSION stays.

## D2735: One compile per load: the loader records the maps in the compile that runs the dump
date: 2026-11-04 · by: sm · issue: 0196
A format 2 load compiled the dump's text to run it and, at the first `$dump.resume`,
compiled it again with recording on for the suspension maps. Recording does not change
the code (the pin test compares the disassembly over the corpus), so the loader now
compiles format 2 text once with recording on (`compiler::compile_with_maps`) and keeps
the maps for every `resume`; format 1 keeps the plain compile. Recording no longer runs
the stack-depth and liveness analyses for functions without suspension sites (they only
contribute their computed keys), and finds the sites' instructions in one pass. Measured
natively over the 1,164 harvested idle-point dumps (23 MB of text, best of 5, release):
plain compile 343 ms, compile + maps (the old load) 711–741 ms, compile with maps 377–381
ms; `load_dump` overall 450–463 ms against about 800 ms before (0.56x). A 12 MB data-only
dump compiles in the same time with recording as without, so recording is not gated on
the text having activations. Alternatives: keeping state tables in templates of a dump
build (a per-function table in the heap for every compile, for a cost recording already
nearly removes), or a per-script map cache across loads (a load is one compile now; the
exporter's compiles at dump time are a separate matter). A test now checks that a loaded
dump is its own fixed point: dumping right after the load writes the same text.

## D2800: String and number constants are not deduplicated per function while parsing
date: 2026-11-05 · by: dumpperf · issue: 0198
The parser deduplicated a function's string and number constants, and the unit's heap
strings, through sorted `Vec`s it inserted into: quadratic in distinct constants (a
dump's data has thousands; 25% of loading a 1 MB records dump). Resolution already
interns every `push_const` of the unit by value (D2446), so the per-function lists only
kept the phase-1 constant list short; they are gone, and the unit's heap strings are
found through an open-addressing index (a string over 256 units hashed by its length and
256 sampled units, compared whole on a match). The bytecode is unchanged (the snapshots
pin it), the slim module is ~140 B gzip smaller, and compile/typescript.js went from 52.8
to 59.9 MB/s. The unit pool (`resolve::UnitPool`) keeps its constants' hashes and hashes
a string over 1 KB by sampling too, for the same reason.

## D2801: With the dumper, pattern scans stop at a literal element
date: 2026-11-05 · by: dumpperf · issue: 0198
The parser scans a `{`/`[` group to its closer to see whether it is an assignment
pattern (docs/compiler.md 3.3), so a dump's data literals were lexed twice. In builds with
the `dump` feature the scan stops, answering "no pattern", at a primitive literal that is
a whole element or property value of a group where a nested pattern could be (`[1, …`,
`{a: "x", …`): no pattern has one there. A valid program compiles to the same code
either way; an invalid one could get another error (`({a: 1} = c)` would report the
assignment instead of the destructuring target), so after a syntax error the source is
parsed again with exact scans (`compiler::parse`), and errors stay what they were. Only
the dump module has it (its loads are where the data is; the shipped modules keep their
size, +470 B gzip otherwise). The records dump loads ~12% faster.

## D2802: One dump-module sandbox per module and memory limit in the process
date: 2026-11-05 · by: dumpperf · issue: 0198
Refines D2701. Each Sandbox with `dumpModule` kept its own dump-module instance, so a
`Sandbox.resume` in packaging B paid a second instance and its VM on top of the new
sandbox (B's resume was ~0.36 ms over A's at the corpus median). The instances are now
shared by every sandbox of the process with the same dump module and memory limit
(`dumpers` in js/src/sandbox.ts): a dump or a load moves the sandbox's image into that VM
and runs no host function there (the loader refuses host calls, the dumper runs no
code), so the VM is the only state, and it is replaced by the next image. B's resume is
now ~0.15 ms over A's at the median. A process keeps one dump instance per limit, sized by
the largest image it took, instead of one per live sandbox.

## D2830: Continuation dumps become plain JavaScript
date: 2026-11-04 · by: owner · issue: 0199
Every suspended activation is a continuation function (the rest of the original from its
site) that is also the factory of its frame's scopes; the dump defines nothing privileged
and runs as an ordinary Script given the host's functions (in djs and in node). Format 2 and
its loader primitives are retired once the plain exporter covers the corpus
(docs/plain-js-dumps.md; issues 0200-0205).

## D2831: The plain dump is written by the Rust exporter
date: 2026-11-04 · by: owner · issue: 0199
The compiler records each site's statement path, block declarations and expression
constructs with spans; the exporter splices the original text. The JavaScript prototype in
work/research/0199 is not shipped.

## D2832: Open host calls and Math.random's state are restored through host functions
date: 2026-11-04 · by: owner · issue: 0199
The dump calls host functions (`resume(id, name, args, state)`, `seed(lo, hi)`), not
engine primitives; the Sandbox tasks provide them from the {call, save, resume} hooks
(D2771).

## D2833: Re-dumps use origin tracking
date: 2026-11-04 · by: owner · issue: 0199
A continuation carries `$djs = {f, at, segs, scaf, cmap}`; the exporter writes the frame
of a continuation as an activation of its origin, so a dump always carries the original
code (pruned of what cannot run again) and its size stays bounded over any number of
re-dumps.

## D2850: Suspension maps carry the syntax around each site
date: 2026-11-05 · by: pjc · issue: 0200
The plain-dump exporter (D2831) writes a continuation by splicing the original text, so
it needs the structure around each site, which the single-pass compiler never builds as
a tree. While recording suspension maps the parser keeps every syntax node that encloses
a site (statements and expressions of the function's own code), with the roles and spans
of its parts and the bindings of the scopes it opens (`FunctionMap::syntax`), and each
site gets its path of nodes from the function body down (`Site::path`), with the part
that holds the next step. Offsets are from the function's start like every other offset
of a map; temporaries `c@o` are found on the path by `o`. Constructs that are not nodes
yet (destructuring patterns, class bodies, TypeScript expressions) leave a *gap* that
`Site::gap` reports (the part holds more than the next node and parentheses), so the
exporter refuses instead of silently dropping a construct. Types and roles are in
compiler/smap_syntax.rs. Alternatives: a real AST for async functions (a second parser
in all but name, and memory for every compile with recording), or re-parsing the
function in the exporter (the dependency D2831 rejects).
## D2890: Plain dumps load as a Script with `$djs.resume`/`$djs.seed`; open calls keep their ids
date: 2026-11-04 · by: pjh · issue: 0204
Makes D2832 exact (agreed with 0201's exporter). A plain dump (`"djs-dump 3";`, or
`"djs-dump 3 typescript";`) is compiled as a Script and run with one reserved global, the
object `$djs`: `$djs.seed(lo, hi)`, the dump's first statement, sets `Math.random`'s state
(today's int32 halves; `Vm::set_math_random_seed`), and `$djs.resume(id, name, args,
state?)`, in place of `$dump.pending`, continues the open call `id` of the host function
the host calls `name`. Both are sync host functions, served only while the Script runs to
its first stop that is not a sync host call (an idle point, or the end), and the dump
deletes `$djs` in the `finally` that ends its Script (with `$dump`), so user code cannot
forge a resumed call later and a re-dump never sees it (the CLI deletes it after the load
too; the shipped module has no export for it, D2892). The exporter refuses runs with globals `$djs` or `$dump`. The host side of
`resume` is an embedding call, not a dump primitive: `Vm::open_host_call(parked, fn_id)`
(`vm_open_call`) reads `id` and `args` from the parked call, returns, opens async call
`id` under that very id and gives its promise as `resume`'s value; the id counter moves
past it (sync calls take ids from the same counter, so the parked call may hold `id`
itself; it returns first). So ids and states survive any chain of dumps with no special
case for `resume` in the exporter (the open call is one of `name`, as it was), and hosts
keep D2771's "same calls, same ids". An unknown name or an id that is open throws at the
call, and the load fails. The Script's completion value is the run's promise (the one
handle, D2633's `run`). Loading needs no dumper: the shipped module without the `dump`
feature loads plain dumps.

## D2891: Hosts load plain dumps themselves; sync host functions run meanwhile
date: 2026-11-04 · by: pjh · issue: 0204
The Sandbox (`load`, `loadRun`, `resume`) and the CLI (`resume`, `--explain`, the
REPL's `.restore`) detect the header and run the dump in their own VM, not through the
dump module. A plain dump runs code (the loaded functions, reactions of settled promises)
while it loads, so its sync host calls are served as in a run (the Sandbox requires them
to return synchronously there, as `begin` does); async calls it makes are new calls the
following run reports. The calls `resume` opened are the loaded calls: the Sandbox lists
them as `OpenCall`s with their states (D2770) and a task continues them by its hooks
(D2771); the CLI answers them like any call and shows their states in `--explain`. A
Script that throws (a refused hook) is a load error, not the run's.

## D2892: The size ceilings rise for plain-dump loading in the shipped module
date: 2026-11-04 · by: pjh · issue: 0204
A trade under D020: `Vm::open_host_call`, `vm_open_call` and `add_loaded_host_call` (no
longer behind the `dump` feature) add 452 bytes gzip to the shipped module (441,355 →
441,807; 1.3 KB raw); the slim module reaches 377,177, over its ceiling of 376,679. They buy resuming
a dump with the shipped module alone: loading a plain dump is running a Script with the
hooks (D2890), so a host needs the dump module (548.9 KB gzip) only to write dumps, not to
continue them. Kept small on purpose: no formatting, no iterator adapters, static error
texts, and no export to delete `$djs` (the dump deletes it, D2890). Ceilings set by the
ratchet's convention, the measured size plus 1 KB: 442,831 and 378,201.

## D2851: Site syntax: member targets split, recording only in async functions and generators, pinned
date: 2026-11-05 · by: pjc · issue: 0200
Refines D2850 after the exporter's review. An assignment or update whose target is a member
has the parts `Object Property` instead of `Left`/`Operand` (the exporter assigns through
the saved object and key), and the member's own node leaves the paths. Recording runs only
in async functions and generators (only they have sites), so a format 2 dump's data, the
top level of a Script, costs nothing; parts are kept in one stack shared by the open nodes
and copied out only for nodes that enclose a site. A left-first construct (binary
operator, call, member, assignment, conditional, sequence, postfix update) opens after its
left operand and is inserted into the paths of the sites recorded since that operand
began. The pin test now prints the nodes and each site's path in tests/smap and checks
that every path starts at the body, names the part of each step and ends at the site's own
construct; tests/smap/summary.txt counts complete paths and gaps by the part they are in
(1,062 of 1,063 corpus sites complete; the one gap is a destructuring default).
## D2893: pi's next mode names its host functions `$djs.bridge`/`$djs.call` in plain dumps
date: 2026-11-04 · by: pjh · issue: 0204
A plain dump writes a host function it holds as the host's name for it, an expression
evaluated while the dump loads (D2890). pi's `bridge` and `call` are no globals (the
prelude gets them as arguments), so the adapter names them `$djs.bridge` and `$djs.call`
for plain dumps and defines them in `$djs` beside the hooks while it loads; each is one
property, so every occurrence is the same function. `$djs.resume(id, "$djs.call", args,
{"id": n})` reopens the call under its engine id, and the adapter's call table takes pi's
id from the state as before (D2772); a call without one fails the load. The adapter
restores its own state (counters, store, clock) before the dump runs, since the dump's
code runs while it loads, and deletes `$djs` itself afterwards as a backstop.

## D2894: The CLI's open calls carry their origin and the answers file's counts as state
date: 2026-11-04 · by: pjh · issue: 0204
`djs run --dump-after`, `--dump-every-idle` and the REPL's `.dump`/`.dump-every` give each
open call a state (D2770): `{"origin": "input 2", "answered": {"read": 2}}`, each field
left out when empty, no state when both are. `origin` is the REPL's label of the input
that made the call, so `.pending` after a `.restore` names it instead of "the dump".
`answered` counts the answers each tool took from the ordered (argument-less) entries of
`--tools answers.json`: run state, not the call's, but an idle point always has an open
call, so every call carries the counts and the loading CLI takes the highest per tool; a
resumed run's next call then gets the answer the uninterrupted run gets, where it used to
start the list again. Read back from format 2's loaded calls and from the plain dump's
`$djs.resume` states alike.
## D2852: Destructuring patterns are site-syntax nodes
date: 2026-11-05 · by: pjc · issue: 0206
Extends D2850 so a site inside a destructuring pattern (a default, a computed key, a
reference target of an assignment pattern) has a complete path. `ArrayPattern` has the
parts `Element`, `Hole` (an empty span) and `Rest`; `ObjectPattern` has `Key Value`,
`ComputedKey Value`, `Shorthand` and `Rest`; a target with a default is a `Default` node
(`Target Init`, the name as `Target` in a shorthand) inside its part when a path goes
through the default or a nested pattern under it. Pattern nodes nest as in the source,
though djs evaluates a pattern after its value; bindings stay on the enclosing node's
`decls`. The pattern's state keeps its existing temporaries, named by the pattern's start
(`pattern.source`, `pattern.excluded`, `pattern.iterator`, `pattern.next`; the
iterator is `undefined` once done) and `pattern.key` by a computed key's `[`. Agreed with
the exporter (0203). pjx had first asked for per-element `PatProp`/`Elem` nodes and
then accepted this shape, which carries the same spans with fewer nodes.

## D2853: TypeScript expressions are transparent on site paths
date: 2026-11-05 · by: pjc · issue: 0206
`x as T`, `x satisfies T`, `x!`, `<T>x` and annotations are not nodes. Each function's
TypeScript-only ranges are recorded (`Syntax::erased`, the ranges the parser already
erases for `strip_types`) and the gap check counts them as blanks, so `(await x) as T`
has a complete path. The exporter splices from `compiler::strip_types`'s text, which
blanks exactly those ranges and keeps every offset, so every span stays valid and the
spliced text is JavaScript. A wrapper node (`Operand` plus `Type`) was the alternative; it
would make every consumer skip it and still leave the types in the spliced text.
Class constructors and `Function`-constructor sources record a `Params` part as well
(neither can have sites today), and a declaration carries `Decl::at`, an offset inside
its declarator, so a statement's names can be told apart by declarator.

## D2895: Several handles in a plain dump: a null-prototype object as the completion value
date: 2026-11-04 · by: pjh · issue: 0204
Agreed with 0201's exporter. A plain dump's Script completes with what the host named:
nothing (no handles), the value itself when the one handle is named `run` (the Sandbox,
pi, `djs run`), else `({ __proto__: null, "input 2": …, "$entry": … })`, every handle by
name in the host's order. By name, not by count: the loading host (the REPL's `.restore`
of a file) need not know how many the dump holds, and a single value may be an object
itself. Loading, the CLI takes a promise as `run` and a null-prototype object's own
properties as the handles (`dumps::plain_handles`); the Sandbox and pi only name `run`.

## D2896: The gate clears a corrupt incremental cache and runs the job once more
date: 2026-11-04 · by: pjh · issue: -
rustc's incremental cache in .worktrees/gate went bad twice: a link failed on symbols no
object defines (`Undefined symbols ... "_anon.<hash>.<n>.llvm.<n>"` in djs_core), which
clearing the incremental directories fixed. scripts/gate.py now recognizes that failure in
a job's log (the linker's undefined-symbols error with an `_anon.*.llvm.*` symbol within
the next 40 lines), removes the `incremental/` directories of the job's target directory
(its group: at the top and one or two levels down, never inside another job's group or a
nested target directory; nothing else builds there meanwhile), keeps the failed log as
`<leg>.log.corrupt`, prints what it did and runs the job once more; a second failure is
reported as usual, and nothing is retried once another job failed. Preferred over
`CARGO_INCREMENTAL=0` for the gate's builds, not measured again: every Rust change
rebuilds djs-core in about nine configurations, and 0047 measured a gate's rustc wall time
at 76 s from scratch (djs-core 49 s) against 20-34 s incremental; without incremental
caches every djs-core edit would pay close to the from-scratch djs-core time in each
configuration, while the corruption is rare (twice) and costs one such rebuild of one
target directory when it happens.

## D2860: The plain exporter writes format 2's lines and an IR, and rewrites them
date: 2026-11-05 · by: pjx · issue: 0201
With `DumpOptions::plain` the exporter (dump.rs, dump_flight.rs) still writes format 2's
lines, which its checker compiles, but in plain forms at the source (builtins as
`$dump.bN` captured first, open calls as `$djs.resume(...)`, host functions inline as the
host's expression for them, `$djs.seed` instead of `$dump.random`), and records an IR
beside them: factories (scopes, variables, members and the variables each captures) and
activations (function text, site, the site's syntax, slots and their cells). The writer
(dump_plain.rs, a port of the 0199 prototype's placement and scheduling) and the
continuation generator (dump_cont.rs, on the site syntax of D2850) turn that into the
plain Script. Choices on the way: a closure of a group a frame shares that data refers to
is an exporter patch after the continuation's call (the prototype's placeholders); the
Script is strict ("use strict" after the header: the functions' texts are strict in
codemode anyway, and the dump's own code needs no sloppy mode); a value a factory writes
by a global's name (`done`) that a variable of the factory would shadow is read before
the factory (`$dump.vN = done;`); dependencies between the dump's statements are what
evaluating them reads, not the names inside function and class bodies (read when they
run, after the load, and a continuation of an earlier dump names that dump's `$dump`);
a continuation's member key is `$K<act>` unless a member has that name already, and a
continuation of a continuation picks scaffold and temporary names its function's text
does not hold. Until 0202 a continuation of a continuation carries the earlier ones as
its `$djs.f`, so chains grow by a few kB per link; the checks cap them at 50 links.

## D2861: Plain dumps are checked over every corpus idle point, in djs and in node
date: 2026-11-05 · by: pjx · issue: 0201
`djs-difftest --plain-dumps` dumps every case at each idle point as a plain dump and
continues in a fresh VM that runs it as a Script, with `$djs.seed` and a `$djs.resume`
that reopens the call under its id (`Vm::open_host_call`, as hosts do, D2890); at most 50
links per case. A refused dump is no failure: the case's dumps end and the run goes on.
`--plain-out DIR` writes every link's dump, the output before it and the uninterrupted
run's transcript; scripts/plain-node.mjs runs every link in a fresh node context with the
difftest prelude's host (output, done, later answered oldest first at each idle point)
and requires the same transcript, with node's clock pinned to djs's, TZ=UTC and V8's
wording of `cannot read property` mapped to djs's (the corpus's only engine-specific
text); TypeScript dumps are not run in node until they are written as JavaScript. Cases
that refuse are a ratchet, tests/plain-refusals.txt (`--update`): a new refusal fails,
one that went away is reported stale. The gate leg "plain dumps (djs and node)" runs both
over the whole corpus (about 1,190 links). The djs-core test
`plain_dump_chains_continue_like_the_run` does the same in-process for the format 2
fixtures (tests/dump/v2).

## D2870: Origin tracking reads a continuation's frame as its origin's
date: 2026-11-05 · by: pjx · issue: 0202
A plain dump of a run that loaded a plain dump holds activations of continuations `K`.
Dumped as they are, each link's continuation would be written from the previous one's
text (its prefix included), and chains grow by a few kB per link. The exporter instead
reads such an activation (a function with an own `$djs = { f, at, segs, scaf, cmap }`) as
one of `f` (dump_origin.rs): `K`'s site maps to `f`'s through `scaf`'s `site` record (K
still at the site it was written for) or the stretches `K` copied (`segs`); the frame is
rebuilt in `f`'s layout from `f`'s suspension map: bindings by name (shadowed ones in scope
order), `arguments` from `$djs_arguments`, `f`'s temporaries of its rewritten statement
from `cmap` (`[slot, 0, "binding of K"]` or `[slot, 1, literal]`, which the generator now
records), a loop scaffold's iterator record from its `$dump.L` wrapper's captures, a
finally's completion from its `scaf` record (`["finally", fo, ko, kind, "binding"]`), the
other temporaries at `K`'s offsets mapped back through `segs`; `K`'s `$djs_*` locals are
dropped. The rest of the exporter sees an activation of `f`, so the next continuation is
written from `f`'s text again and what only `K` held (helpers, the earlier origin
records) is not dumped. For that `f` must be a function whose template the exporter can
read maps from: `K.$djs.f` is the run's function when the dump reaches it by name, else
`$dump.F<act>`, made from its text by the dump (where it is never called). A generator's
continuation takes the generator object's prototype (`Act::proto`), not `f`'s
`prototype`. Refused, for 0203: a continuation that stopped again inside its own rewritten
statement, a for-in scaffold, a codemode body held by reference. Measured over the corpus:
a link is the size of a dump of the uninterrupted run at the same point (the djs-core
chain test checks it within 512 bytes per link), and the corpus's plain dumps total 24.5
MB instead of 36 MB.
## D2897: `$djs.seed(lo, hi, next)`: call ids go on after a plain load as in the uninterrupted run
date: 2026-11-04 · by: pjh · issue: 0204
Refines D2890. The hooks are sync host calls, and sync calls take ids from the same counter
as async ones, so after a plain load the run's next calls got ids one or two higher than
the uninterrupted run's (format 2 restored the counter in its loader). Ids are host
visible and journaled, so the dump now says where the counter stands: the exporter writes
the dumped VM's next call id as `$djs.seed`'s third argument, and a djs host, once the
Script has stopped (its first idle point, or the end), sets the counter to the larger of
it and one past the highest id `$djs.resume` reopened (`Vm::set_next_call_id`,
`vm_set_next_call_id`; the Sandbox, the pi adapter, the CLI and the difftest do). Non-djs
hosts ignore it with the rest of `seed`. The CLI also had ids drift from its lazily made
tools: `tools.read` read after a load called `make`, a sync call, where the uninterrupted
run had made the tool before the dump; its call states (D2894) now list the tools made
(`made`, in their id order), and a load puts them back into the `tools` Proxy's table
without a call. Checked by the CLI's plain every-idle-point test, which compares the
hook calls (ids, states, seed) of each resumed dump's next dump with the uninterrupted
run's dump there, by js/test/dumpresume.test.ts's plain variants, whose transcripts list
the ids, and by pi's adapter test, whose messages carry pi's `issued` counts.
## D2898: The Sandbox writes plain dumps by default
date: 2026-11-04 · by: pjm · issue: 0204
`SandboxOptions.plainDumps` defaults to true: new Sandbox dumps (tasks' `suspend`/
`checkpoint`, `Run.dump`, `dump`) are plain Scripts (D2830). The owner wants the plain
format first, and format 2 is retired in 0205 anyway, so keeping it the default until
then buys nothing. `plainDumps: false` still writes format 2; the tests that assert
format 2's text (dumpload, task, dumpresume's format 2 packagings) opt into it until
0205 rewrites them. The CLI (`--plain`) and the pi adapter (`plainDumps`) keep format 2
as their default until 0205.

## D2871: Plain continuations of methods that use `super` are methods of the home's prototype
date: 2026-11-05 · by: pjr · issue: 0203
A continuation `K` of a method that uses `super` is written as `({ __proto__: P, async
K() { … } }).K`, P the original home object's prototype, and called with the original's
`this`, so `super[…]` and `super.x` in the rest of the method read what the original
reads. A `super[<site>]` in flight writes `super[` again; the exporter refuses the dump
when a base or `this` read before the site is no longer what `super`/`this` give (a home
object whose prototype changed since). `K`'s `segs` and `scaf` count from the method's
own text (what origin tracking reads back), not from the wrapper. `new.target` stays
allowed in async functions and generators (never constructed: undefined in `K` too), and
is refused in arrows.

## D2872: `for await` in plain continuations: a loop over the saved record, the iteration in flight before it
date: 2026-11-05 · by: pjr · issue: 0203
A `for await` around a site is written as the rest of the current iteration in a loop
that runs once (so `continue` ends it and `break` leaves it, recorded in a state
variable), inside `try`/`catch`/`finally` that close the iterator as the loop would
(AsyncIteratorClose: `return()` awaited and its result checked after a `break` or a jump
out; on an exception a promise that always fulfills, `$dump.X`), followed by the loop
itself over the saved record (`$dump.N(it, next)`). A real `for await` around the
iteration in flight would add an await before its rest (the first `next()` is always
awaited) and change job order. At the loop's `next` the loop is `for await` over
`$dump.N(it, next, awaited)`, whose first step is the awaited promise. At a close site
the statement is `$dump.O(await p)` (or `await p; throw e`) and the exit taken after it:
the completion's kind from the frame, else the loop's close sites (exception, jump out,
`break`) say which; a single labelled jump with no kind in the frame is refused. The
async-from-sync iterator is the dump's `$dump.fromSync(it, next)`, a plain object whose
`next`/`return` step as %AsyncFromSyncIteratorPrototype%'s do (the object is never
visible to user code), and its close-on-rejection handler is `$dump.closeThrow(it)`.
The record's iterator and `next` are constants of `K` named in the scaffold record
(`["for-await", fo, ko, 0, it, next]`) for origin tracking; a site record may name the
origin's site (`["site", fo, ko, "for-await.close#2"]`) where `K`'s own site is of
another kind. An activation whose continuation is defined in another activation's prefix
always waits for it: the scheduler's cycle breaking never moves that dependency into a
callback.

## D2873: Origin tracking inside a continuation's rewritten statement, case tests, finally by jumps
date: 2026-11-05 · by: pjr · issue: 0203
A continuation that stops again inside its own rewritten statement is read back as its
origin: the `await`/`yield` keywords there are copied from the original (so the site
maps through `segs`); the original's temporaries come first from what `K` copied (a later
loop iteration runs the original's text), then from `K`'s own constructs named in
`["tmp", 0, ko, "call.arg@424", "array.element"]` records (an argument or element `K`
evaluated into its own array literal), then from `cmap`'s constants. `K`'s own call
(`$djs_R(`, its callee, `this` and fixed arguments) is marked with `["k", 0, ko]`
records: those temporaries are rebuilt from the origin's frame, so they no longer make
origin tracking refuse. A completion kind that names a statement by offset (break,
continue) is translated from `K`'s offsets to the original's. A finally entered by
`break`/`continue` ends with the jump: unlabelled where the target is the innermost loop
(or switch) around it, since `K`'s scaffolds nest like the original's statements, else
by its label. A site in a case test writes the switch again over the saved discriminant
with the tests that already failed as `case {}:` (a fresh object, never equal); a site in
the discriminant rewrites it in place. A for-in scaffold's `$dump.forIn` iterator is read
back by its closure's captures.

## D2874: Destructuring around a continuation's site is written as a pattern again
date: 2026-11-05 · by: pjr · issue: 0203
The rest of a pattern whose evaluation stopped is a pattern again over a source that
goes on where the original stopped (dump_cont_pattern.rs). An array pattern goes on over
the saved record: `$dump.L(it, next)`, whose placeholder first step makes the element's
default run (the site's default, or the rest of a nested pattern written as that
default), `$dump.I(it, next)` where the element's step is still to come (the site is in a
reference target, evaluated before the step), and `$dump.E()` once done. The language
closes the iterator as the original would. An object pattern becomes `{ v0: …, v1: …,
vR: rest }` over an object of the dump's whose getters read the saved source when the
original reads it. Computed keys are evaluated in the pattern's own computed keys,
converted once (`$dump.P`, ToPropertyKey as a computed key does it), and appended to the
excluded keys at the key, as the original does. The rest property is `$dump.S(src,
excluded)`, CopyDataProperties without the keys taken. Bindings the pattern initialized
before the site are declared with their values (a binding's `at` lies in a part before
the site's). An assignment pattern is `((pattern = source), value)`. The loop wrapper's
`return` is now a getter that looks up the iterator's own, so a missing one is not
called and the call gets the arguments the language passes. Origin tracking reads K's
pattern state back: its array pattern's iterator (`["pattern", fo, ko]`), its own
reference targets and `$dump.P` calls (`tmp`/`k` records), the keys as K's variables.
Constants that K's closures capture are unboxed. Several sites of one kind at one
construct (a `for await`'s closes) are matched by their place among K's. Scaffold marks
of `for`/`while` loops now sit at the keyword, after a label, because a jump's completion
names the loop by its keyword.

## D2834: Plain dumps are mostly self-contained: the code to continue is in the dump, the host provides `$djs`
date: 2026-11-04 · by: owner · issue: 0207
A dump carries everything needed to continue the run: data, closures, continuations, open
calls. The host provides one documented object, `$djs`: `seed` and `resume`, the format's
fixed helpers (today written into every dump as `$dump.R/L/advance`; they are the same code
in every dump, a small standard library of the format, not program content), and
`source(ref)`, which returns the text of a top-level script the host ran. A dump refers to
that script's origin by reference instead of writing it a second time; only dumping again
needs it, and a host that cannot supply it gets a clear refusal (a host that wants
standalone dumps asks for the text inline). No magic: the interface is small, written down,
and the same for djs hosts and node (which loads a shipped runtime file). `$djs` is a global
for now; it is meant to become imports later, so its members are plain functions that do not
depend on `this` or on each other through the object.

## D2835: The dump's first line is `"djs-dump";`
date: 2026-11-04 · by: owner · issue: 0207
djs has never been released, so the format carries no version number (D024); the TypeScript
variant is `"djs-dump typescript";` while function texts can still be TypeScript.

## D2900: A plain dump's `$djs` record leaves default fields out
date: 2026-11-04 · by: pjs · issue: 0207
`Object.defineProperty(K, "$djs", { value: { f, at, segs, scaf, cmap } })` is written
with only the fields that differ from their default: `at` (informative, read by no one)
when the site is not `await#1`, `segs`, `scaf` and `cmap` when they are not empty. The
reader (origin tracking) takes a missing array as empty. Records are the part of a dump
that gzip helps least (numbers); this saves ~3% of a pi session's dump.

## D2901: The plain dump size score
date: 2026-11-04 · by: pjs · issue: 0207
tests/scores/dump-size.tsv holds the total raw and gzip -9 bytes of the plain dumps at
every idle point of the corpus (difftest `--plain-out`) and of the v2/golden programs
(`plain_dumps_at_every_idle_point` with `DJS_PLAIN_OUT`), measured by
`scripts/dump-size/dump-size.ts --score`. The corpus counts without its test environment
(`env`: the difftest prelude's state and the codemode-harvest harness with its synthetic
tool results, 64% of the corpus's bytes), which a pi session never dumps. The gate leg
"plain dumps (djs and node)" fails when a total grows beyond a small tolerance (the wall
clock's digits in the dumps, zlib versions); a total well below its line asks for
`--update`, so the score is a ratchet.

## D2902: Anonymous closures in a scope factory are written as `key: (0, f)`
date: 2026-11-04 · by: pjs · issue: 0207
A closure that is a member of its scope factory's returned object (`$f33: (args) => …`)
would be named by the key, so the dump fixed its `name` back to `""` afterwards with
`Object.defineProperty(f, "name", { value: "" })`. An anonymous function whose own `name`
is the empty string is now written `$f33: (0, (args) => …)`: not a direct function
definition, so the key does not name it, and no fixup follows (−9% of the corpus's
plain dump bytes, mostly the prelude's closures).
## D2903: The plain dump runtime: the format's helpers are members of `$djs`
date: 2026-11-04 · by: pjs · issue: 0207
The helpers a plain dump calls (`R`, `L`, `advance`, `fromSync`, … ; the full list with
their meaning is in docs/plain-js-dumps.md §4) are no longer written into each dump as
`globalThis.$dump = { R: …, L: … }`: the host provides them as members of `$djs`, next to
its hooks (D2834), and the dump calls `$djs.H(...)` (a continuation captures the ones it
calls, `const $djs_R = $djs.R`, since `$djs` is deleted at the end of the load). They are
the format's fixed standard library, not program content. One table,
crates/djs-core/src/vm/dump_runtime.rs, defines them: `Vm::dump_runtime()` evaluates it in
a VM (the CLI, the difftest), and js/src/dump-runtime.js, generated from it and kept current
by a test, is the ES module for hosts without djs (one named export per helper, and
`helpers`) and holds the same as a Script (`script`), which the Sandbox and pi's adapter
pass to the wasm export `vm_dump_runtime(vm, len)` (`Vm::dump_runtime_from`): the shipped
module does not carry the text (it would cost ~2 KB gzip of the size budget). Each helper
is a plain function that uses neither `this` nor another member, and the names stay short
and map 1:1 to named exports, so that `$djs` can become imports later. Format 2 keeps its
own `$dump` helpers. Cost: a re-dump of a loaded dump writes objects the runtime made (a `for await`'s `fromSync`
wrapper) with the runtime's code as closures, where the run had the engine's own record.

## D2904: The hosts' plain-dump paths run with frozen builtins
date: 2026-11-04 · by: pjs · issue: 0207
How D2836 (plain dumps only with frozen builtins) reaches the hosts. The exporter refuses
a plain dump of a VM without them. `djs run --plain` (and `resume --plain`) and `djs
repl --plain` make their VMs frozen; `.plain on` in a REPL started without it says that
its dumps will be refused. The Sandbox's `plainDumps` default follows the VM's mode
(`vm_frozen_builtins`, so a restored sandbox keeps it): plain with frozen builtins, format
2 without, until 0205 retires format 2 (after which a sandbox without frozen builtins
cannot dump). pi writes plain dumps only in its `next` mode, which is frozen. The
difftest's plain-dump variant runs every case frozen and compares it with the plain run
only when the plain run passed an idle point: 24 corpus cases change builtins on purpose
(builtins/, language/, errors/, the legacy decorator cases of ts/), none of them takes a
dump, and their frozen output differs for that alone. The core plain-dump tests use frozen
VMs (`plain_vm`).

## D2905: A continuation reads an evaluated operand again where that is exact
date: 2026-11-04 · by: pjs · issue: 0207
The exporter's rule for D2836's builtin operands (dump_reread.rs). A call's callee and
`this`, evaluated before the site, are written as the original text (`results.push((await
p))`, no `$djs_R` and no temporaries) when the callee is an identifier holding the saved
callee (and `this` was undefined), or `b.name` where `b` is an identifier holding the saved
`this` and the lookup of `name` on it reaches the saved callee as a data property of a
frozen object, through frozen objects only after the receiver, which must not have the
property itself unless it is frozen. An identifier "holds" a value when its binding does
now: the frame's binding of that name (only one of the name in scope at the site), else a
variable the function captured, else the global lexical record's or the global object's
data property. An argument evaluated before the site is its text again when it is such an
identifier. Everything else keeps its temporary (a receiver reassigned after its read, an
own method of a session object, a getter, a proxy, a spread before the site). The check
runs on the dumped state, which is the state the continuation starts in, so no check is
written into the dump. Origin tracking needs nothing new: K's call is a copy of the
original's text, so K's own temporaries map to the original's through `segs`; K's own
arguments (the site's, a kept temporary) are `tmp` scaffolds of construct `call.arg`.

## D2906: The dump size score is per case
date: 2026-11-04 · by: pjs · issue: 0207
D2901's score recorded one total per suite, so every new corpus case raised the totals and
hid real growth elsewhere. tests/scores/dump-size.tsv now has a line per case (`SUITE CASE
RAW GZIP`; a case is the directory of its dumps). A case that grows past its line (by more
than 8 bytes and 0.1% raw or 0.5% gzip) fails the gate leg; a new case, a removed one, or
one that shrank only asks for `--update`, which rewrites the suites' lines.

## D2907: A continuation's callback runs in parts, at the ends of the blocks it needs
date: 2026-11-04 · by: pjs · issue: 0207
A plain dump's callback (`$dump.gaN()`, what the dump makes inside a continuation because
it needs the closures, classes or continuations the continuation's blocks make) ran once,
at the first block with a binding that takes its values, else at the site's statement.
pjz's fuzzer found both ways that fails: a loop scaffold's head (`const $djs_i1 =
$dump.a9` of a `for await` over a suspended generator) read a value the callback made only
later, at the site; and a callback placed after the top block needed a continuation a
loop's block made later. Now the callback is split by the continuation's block each of
its entries needs (the latest block that makes a factory key the entry, or an entry it
depends on, uses): part `b` (`$dump.gaN_b()`) runs at the end of block `b`'s code, before
the bindings of `b` that take its values and before everything nested in `b`, so loop
heads and bindings see what the callback made for their block. Statements and calls keep
their order among themselves. A binding whose value only a deeper block's part makes is
refused.

## D2908: A for-of or for-in goes on after the rest of its iteration, where nothing tells
date: 2026-11-04 · by: pjs · issue: 0207
A continuation stopped inside a `for...of` or `for...in` wrote the loop as a placeholder
scaffold (`{ let f = true; for (const v of $djs_L(it, next)) { if (f) { f = false; rest }
else { const x = v; body } } }`), so that `break`, `continue` and the iterator's closing
behave as in the original. Neither can tell the rest apart from an iteration when the body
has no `break` or `continue` (a conservative scan of its text) and the iterator has no
`return` (found by a lookup through frozen builtins, D2836: arrays', strings', maps' and
sets' iterators, a for-in's keys): then the rest runs first, in a block that declares the
iteration's bindings, and the loop goes on as written over the saved record, `for (const x
of $djs_I(it)) body` (`$djs.I` takes the iterator's own `next` when the dump leaves it out,
which it does where `it.next` reads the saved one). The record is bound to K's constants
before the rest, which origin tracking reads back while K is in the rest (`["for-of", fo,
ko, 0, it, next]`); inside the loop it unwraps `$djs.I` as it did `$djs.L`. A dump also
writes only the builtins (`$dump.bN`) its text reads.

## D2909: A continuation is a function called with the frame's values
date: 2026-11-04 · by: pjs · issue: 0207
A continuation declared every live binding of the function's body with its value (`var
ms = 5000; var step = $dump.n2;`), was stored (`$dump.Ka6 = async function () {…}`), given
its origin record with `Object.defineProperty($dump.Ka6, "$djs", { value: {…} })` and
called with `.call(undefined)`. Now the body's bindings are K's parameters and their
values its call's arguments, and K is written where it is called: `$dump.a6 =
$djs.k(async function (ms, step, t0, results) {…}, { f, segs, … })(5000, $dump.n2, …)`
(`.call(this, …)` when `this` is not undefined; `$djs.k`, a runtime helper, sets the record
and a generator continuation's prototype and returns K; a continuation a factory holds
keeps its reference). A binding stays a declaration where a parameter would differ or
cannot be: a variable shared with closures K's blocks make, one whose value the dump
makes later (after its callback, or from a factory key), a `using` resource, `arguments`,
and a constant the text may assign (a conservative scan: an assignment or update operator
at an occurrence, an occurrence before `of`/`in` or in brackets an assignment follows),
whose assignment must still throw. The arguments are evaluated where the call is, outside
K, so the values that read names K shadows need no `$dump.aN_k` copies any more. K also
leaves out blank lines of its own and the braces of a block that declares nothing where
it stands among statements. Origin tracking reads parameters as K's bindings by name, as
before.

## D2910: A continuation reads a value of the callback only after the part that makes it
date: 2026-11-04 · by: pjs · issue: 0207
D2907 placed the callback's parts but not what K reads of them: a binding declared at its
block's start could read a value a part made only at that block's end (an instance of a
class of the block, `const o = $dump.n11`), and a loop scaffold's constants a value a part
of a deeper block made (`const $djs_i1 = $dump.a15`, the generator's continuation made with
a later activation, since calls keep their order). The writer now knows which part makes
each value of a callback (`made`); a binding of block `b` whose value part `b` makes is
declared after that part, one whose value a deeper part `p` makes is declared without a
value and assigned right after part `p` runs (a constant becomes `let` only where the text
cannot assign it), and so are a scaffold's constants; a parameter never takes a value of
the callback. A for-of written as the placeholder scaffold, whose iterator the head must
read, refuses such an iterator, and an assignment that a deeper block's binding of the
same name would capture is refused.

## D2911: A continuation leaves out the frame's bindings nothing in it reads
date: 2026-11-04 · by: pjs · issue: 0207
(The issue's proposal C.) After K is generated, a binding of the frame that K declares
(or takes as a parameter) and that nothing in its scope of K reads is left out, with its
assignments; a block of K whose declarations are all gone loses its braces. "Reads" is a
scan of K's text in the binding's scope for the name, so closures K's blocks make count,
and a name used anywhere in the scope keeps the binding. Not left out: a binding whose
name the frame has twice (`name@scope`, which origin tracking needs to match), K's
`$djs_arguments`, `using` resources (their block is their disposal scope), K's own
scaffold bindings, and everything when K's text names `eval` or `with`. On a re-dump,
origin tracking reads a binding K left out as one not yet initialized, a parameter as
undefined, so the next dump writes no data for it; a link of a chain may therefore be
smaller than a dump of the run at that point. The size score's scanner counts the
harness's synthetic results as environment also when the dump writes them in place.

## D2912: A continuation writes origin records only where it can stop again, and less text
date: 2026-11-04 · by: pjs · issue: 0207
From a line-by-line review of the sleep-dump example's dump. The records of K's own
constructs (`k`, `tmp`), which origin tracking needs only when K stops again inside its
rewritten statement, are written only where the construct can: its text after the site
has an `await` or a `yield`, or one lies between it and the site. A block's end is a site
(for its `using` disposal) only where the block or the loop head declares `using`. Text K
writes that goes on the original's right after a copied stretch (` of `, `const r = `), and
a copy right after another, extend that stretch instead of starting a segment. The site is
written without parentheses where an assignment expression may stand (an argument, an
element, a property's value, an initializer, an expression statement, a `return`'s or
`throw`'s operand, an assignment's right side, a template's substitution). A placeholder
loop whose iterator the dump's callback makes inside the loop's first iteration takes the
record late (`$djs.L`'s `set`, after that part ran; D2910 refused it).

## D2875: `yield*` in plain continuations: a `yield*` over a wrapper of the inner record
date: 2026-11-05 · by: pjr · issue: 0203
At a `yield*` site, `K` delegates again with `yield* $dump.Y(it, next, async, ...first)`.
The wrapper's first step is the inner result the original waits for: the promise of
the inner `next()` or `throw()` whose result an async generator awaits. Where the
original is suspended at the delegation's yield, the first step is a placeholder `K` is
primed to. Later steps call the saved `next`. `throw` and `return` are getters that look
up the inner iterator's own when the delegation uses them, so a missing `throw` closes
the iterator and throws as the original's delegation does. An async generator waiting
on the inner `return()`'s result is refused, because the result decides between a return
and a yield. The dump's async-from-sync iterator gained `throw`. Literals rewritten as
`$dump.D` calls mark them for origin tracking, and their computed keys are converted
first (`$dump.P`), as the original converts them before the value. An array literal of
`K` that appends maps to the original's.

## D2836: Plain dumps are written only for VMs with frozen builtins
date: 2026-11-04 · by: owner · issue: 0207
Only the frozen-builtins mode (D022, D2304; pi's next mode has it, D2652) needs dumps, so
the plain exporter requires it and refuses otherwise. It may then assume no builtin changes
between the original evaluation and the load: an already-evaluated operand that resolves to
a builtin (`results.push`, an iterator's `next`) is written as its original expression or
its builtin path, with no saved temporaries and no checks.

## D2876: `using` in plain continuations: the resources added again, a disposal as a scope of K
date: 2026-11-05 · by: pjr · issue: 0203
A plain dump writes no `using` record. The exporter hands the continuation the scope's
`[value, method, kind]` triples and its disposal state (`Act::usings`). A site in the
scope's code: `K` declares the `using` bindings as constants and adds the resources
again at the start of its scope, in their order, as `using`/`await using` of
`$dump.U(value, method, kind)`, an object whose dispose method calls the saved method
on the saved value; a `null` resource is `await using … = null`. The language then
disposes them as the original would. A site in the disposal itself: `K` writes a scope
that adds the resources not yet disposed, and last (so disposed first) the one being
awaited: its awaited value again, the await a `null` resource causes (`null`), or the
exception a sync disposal method of an `await using` threw. The scope completes as the
original did (the carried exception thrown, else its return or jump), so suppressed
errors and the remaining awaits follow from the language. Origin tracking maps `K`'s
scope record to the original's (`["using", fo, ko]`, at `K`'s first resource binding)
and reads `$dump.U`'s closures back (`$djs_v`, `$djs_m`). The disposal sites at a block's,
an iteration's and the body's end are site records. A disposal waiting for a `null`
resource's await with `needsAwait` set is refused.

## D2877: Iterator helpers in plain dumps are made again by their public methods
date: 2026-11-05 · by: pjr · issue: 0203
A plain dump makes an iterator helper again with its public method over its saved state.
The underlying record is `$dump.I(it, next)`. A callback that already saw `k` values is
`$dump.M(f, k)`, which calls `f` with the counter shifted. take's and drop's remaining
counts are their limits, Iterator.from's wrapper is `Iterator.from($dump.I(…))`, and a
concat's unopened items are `$dump.CI(method, iterable)`. A flatMap inside its inner
iterator and a concat inside an item are made one step before and stepped once while the
dump loads (`$dump.FM`, `$dump.CC`). That step yields a placeholder, and no user code
runs. A completed helper is `$dump.HD()`. The exporter reads these wrappers back
(`next`'s captures, `$djs_f`/`$djs_k`, `$djs_m`/`$djs_o`), so re-dumps write the
original's state again. chunks, windows and zip helpers in flight are refused.

## D2878: Plain dumps refuse instances that carry private names
date: 2026-11-05 · by: pjr · issue: 0203
An object with a class's private field, method or accessor cannot be written as plain
JavaScript: only the class's own construction adds a private name to an object. Making
that possible would mean rewriting the class's text in the dump (initializers and
constructor bodies that do nothing while the dump loads, static methods injected to set
the values and deleted afterwards). That is the machinery plain dumps exist to avoid, and
it would show in `Function.prototype.toString`. Such instances that are live across an
await are rare in codemode, so the exporter refuses them with a message that names the
class and the private name. A class's static private members, which its evaluation
installs, are unaffected. In the same change, the continuation of a method of a class
that a factory makes (`$dump.s2.Tagged.prototype.tag`) is placed in the factory, next to
the class, so it sees the same variables.

## D2879: TypeScript plain dumps are written as JavaScript where the TypeScript is only types
date: 2026-11-05 · by: pjr · issue: 0203
A plain dump that holds functions of TypeScript scripts is written as JavaScript. The
finished dump is parsed with the TypeScript grammar, and every erased range (annotations,
`as`, `satisfies`, `!`, type arguments, type-only statements) is blanked with spaces.
Offsets do not move, so the continuations' origin records (`segs`, `scaf`, `cmap`) stay
true, and origin tracking reads the blanked texts as it reads the originals. The header is
then `"djs-dump";` and node runs the dump. If the result does not compile as JavaScript,
because the TypeScript is more than types (an enum, a namespace, parameter properties),
the dump keeps its TypeScript and the `"djs-dump typescript";` header. djs runs that
dump, node does not, and scripts/plain-node.mjs counts it separately. This replaces
D2861's "TypeScript dumps are not run in node". Origin tracking also gives a function's
`this` local the continuation's `this`, because `K` is called with it.

## D2880: Sites in loop heads, `var` declarations, catch parameters and more literals
date: 2026-11-05 · by: pjr · issue: 0203
More sites get plain continuations. A site in a `var` declaration is written as
assignments, because `K`'s prefix declares the names. `for (var …)` heads are assignment
targets. A site in a `while`/`do` test is `while (first ? (first = false, <rest of the
test>) : <test>)`. A site in a `for` loop's test or update does the same in the loop's
test position, with the loop's bindings from the frame. A site in a `for` loop's init
writes the init with the site rewritten, and one in a `for-in`/`for-of`/`for await`
iterable rewrites the iterable. In an object literal, a computed key holding the site is
converted with `$djs.P`, and methods and accessors after the site are defined with their
descriptors from a literal of their own (`$djs.DD`). A logical assignment to a member at
its right side is the store. An update of a member whose object or key holds the site
writes the reference again. A tagged template is the call `K` makes with the saved tag,
`this`, template object and substitutions; the template object is the dumped copy, so it
is not identical to the one the realm caches for that location. A site in a `catch`
parameter's pattern declares the parameter in a block of `K` with the rest of the
pattern, followed by the catch block.

## D2881: Classes in a scope a waiting frame shares go into the continuation's block
date: 2026-11-05 · by: pjr · issue: 0203
A class whose methods read variables that a suspended frame shares (a class declared in
a codemode body or an async function, its methods using the function's `let`s) is
written in the continuation's block, where those variables are declared. It goes to the
innermost block among the variables its text names, as a closure does, and the members
that name the class go to the same block, right after it, so a subclass placed later can
extend it. A class statement `class C {…}` would declare `C` a second time next to the
frame's binding, so it is written as an expression (`let $djs_c… = class C {…}`). Its
own inner binding `C` still names the class, and the frame's `C` takes the member as
before. A class that names none of the scope's variables, or whose members land in
different blocks, is refused.

## D2883: Classes that name themselves dump plainly; a class's own static private fields
date: 2026-11-05 · by: pjr · issue: 0207
The plain writer's scan of what a statement reads counted a class's own name (`class
A {…}`) as a read of the global `A`. A factory that makes a class whose methods name it
then depended on the global the dump defines from that factory, which reported "cyclic
dependencies". The name after `class` binds and reads nothing. A frame's binding of a
private name (`#x`, which the class's methods capture) is dead like the class's other
hidden bindings: the class written from its text makes its own. A class's own static
private field still holds the literal its text initializes it with (`static #n = 0`)
needs no patch, because the class's evaluation sets it. One that changed since is
refused with a message that names the class and the field (only the class's code can set
it, D2878); before, the message called the class an instance.
## D2960: Exception handlers in unreachable code are dropped
date: 2026-11-05 · by: pjk · issue: 0210
An array pattern's close handler (and `yield*`'s throw handler, and the iterator closes
of `return` inside a pattern) gets its stack depth relative to a label, read from the
final code's stack walk. In code after `return`, `throw`, `break` and the like that
label has no depth. So in a function with such handlers, the compiler finds what is
reachable from the entry alone, adding a handler as an entry point of the walk only
once code in its region is reached, and drops every handler that never was (absolute
ones too). They can never fire, and keeping them would make the stack checker enter
their targets at a made-up depth (a `yield*` throw path rejoins its loop and would
underflow), while a dead absolute handler would make the dead pattern region around it
look live. Functions with no relative handler keep their table as emitted and are not
walked. Not chosen: skipping dead statements in the compiler (early errors and hoisting
still need them parsed, and the region bookkeeping is spread over many emitters), or
giving dead handlers an arbitrary depth. A first version kept the old loop and ran this
pass only when it stalled; the single loop is 570 bytes smaller in the slim module.

## D2884: Re-dumps through loops, finally blocks and `using` scopes
date: 2026-11-05 · by: pjr · issue: 0207
Several re-dump refusals that pjz found by fuzzing are fixed. A function declared in a
block is declared in `K` with `let` (strict code: it belongs to the block), so origin
tracking finds it in the block where the original has it, not hoisted to `K`'s function.
A `using` scope's disposal site right after a `}` that `K` copied maps to the original's
end of that stretch. A finally scaffold's completion record applies only while `K` is in
the scaffold (its constant is in scope); in a later iteration the copied `finally` is
read like any copied text. A disposal continuation declares the scope's bindings first,
so a function of the scope that shares a `let` has its variable. A continuation placed in
an unmerged factory written inside another continuation's block waits for that
activation (the callback), as one placed in the block does.
## D2940: A seeded fuzzer of codemode-like programs checks plain dumps from every idle point
date: 2026-11-05 · by: pjz · issue: 0208
scripts/plain-fuzz/gen.mjs writes small async programs from a seed: tool calls awaited in
literals, calls, member targets, destructuring, templates, conditionals and logical
operators, every kind of loop, try/catch/finally, switch, labels, closures, classes,
accessors capturing frame variables, generators, async generators, `Promise.all` over
mapped async functions and `using`. Values stay small and typed, so a program runs
without errors of its own and prints what it computes. scripts/plain-fuzz/fuzz.mjs runs
each through the difftest's plain-dump variant and node (plain-node.mjs) with a chain of
dumps starting at every idle point: the difftest's `--plain-from K` leaves the idle points
before K undumped, so that each site is also dumped from its original function, not only
from a continuation (a chain from idle point 1 writes every later site of the codemode
body from `K`). Failures are grouped by signature (the reason, numbers blanked); `reduce`
deletes lines and replaces bracketed groups while the signature stays. It is a tool with
a seed and caps on programs and seconds, not a gate leg: its findings become corpus cases.

## D2941: `difftest: plain-xfail <reason>` marks a known plain-dump bug in a corpus case
date: 2026-11-05 · by: pjz · issue: 0208
A corpus case that reproduces a plain-dump bug which is not fixed yet (a wrong result or
a crash of a dump's continuation, in djs or node) carries the directive with a one-line
reason. Its plain-dump variant still runs; a difference is not a failure but is counted,
and once djs and node both continue identically the mark is reported as stale (the
difftest without `--plain-out`, else plain-node.mjs, which learns the djs side from a
`plain-xfail` file the difftest writes next to the transcript). Bad refusals go to
tests/plain-refusals.txt instead, which already is a ratchet. A plain-xfail case still
runs every other variant and its `.expected`.

## D2885: Re-dumps inside K's own operators, templates, callees and assignments; helper classes in bodies
date: 2026-11-05 · by: pjr · issue: 0207
A continuation that stops again inside its own rewritten statement is now read back for
more constructs. Where the original's operand sits in a construct of `K`'s own, `tmp`
records name it: a binary operator's left operand, a template substitution's value, a
call's callee and `this` whose key awaited. `k` records mark `K`'s own constructs whose
temporaries origin tracking rebuilds: the operator after a saved left operand, an
assignment's target, a compound assignment's operator. A class's inner name binding (the
`C` its own methods use) is no variable of the factory, because the class's text makes it
again. That kept such a class from a scope of its own, which neither format could write
("a class in a nested scope of shared variables"). A class whose outer binding a waiting
frame shares is declared by the continuation as that variable ("a cell without its
variable").

## D2920: Format 2 is retired; plain dumps are the only format, and the goldens are plain dumps
date: 2026-11-05 · by: pjf · issue: 0205
D2830's last step. Deleted: `Vm::load_dump` and its options and results (vm/dump_load.rs;
version 1 dumps too, tests/dump/v1.js), `$dump.resume` (vm/dump_resume.rs), the loader's
primitives (vm/dump_prims.rs), the recording compile of a dump's text at load (D2735),
`DumpOptions::plain` (every dump is plain; a VM without frozen builtins is refused with
"dumps need frozen builtins", D2836), the format 2 text the exporter assembled before
rewriting it, the hand-written format 2 fixtures (tests/dump/v2/*.dump.js) and the
two-commit check (scripts/dumps-across-builds.sh, scripts/dumpx). The golden dumps of
tests/dump/v2/golden/ were rewritten once, as plain dumps of the same programs at the
same idle points (class-fields, whose one idle point is refused under D2878, has none and
is listed in plain-refusals.txt); they are run, not loaded, so they hold the dump runtime
and the engine to the format and no longer the exporter or the compiler. D2619 still
applies: a golden dump changes only by a decision. The suspension maps stop being a
format contract (D2563's pins stay, as the compiler's regression test). The hosts lost
their format 2 paths in the same issue: the Sandbox's and pi's `plainDumps`, the CLI's
`--plain` and `.plain` (`djs repl --frozen` makes the REPL's VMs dumpable; `run`'s dump
flags and `resume` make frozen VMs), wasm's `vm_load_dump` and `vm_dump`'s options line
(ABI 4), the difftest's format 2 `--dump-every-idle`. The e2e dump mode runs its cases with
frozen builtins and compares only the dumps' round trips. Freed (gzip -9, `wasm` profile):
the dump module 676,996 → 657,347 bytes (1,667,546 → 1,616,357 raw) over the issue, with
0203/0207's merges in between; the shipped module 442,456 → 442,358, slim 378,026 →
377,888 (the loader was never in them; the rest is D2921).

## D2921: The `jump_if_loading` opcode is removed
date: 2026-11-05 · by: pjf · issue: 0205
D2580 made classes skip their static fields and blocks while `Vm::load_dump` ran a format
2 dump, which restored their effects itself (`heap.loading_dump`, `jump_if_loading`
around a class's static initializer). Nothing sets the flag once the loader is gone, so
the opcode, the flag and the compiler's jump went with it; later opcodes are renumbered
(images are keyed on the descriptor, D2265). A plain dump evaluates a class's text
again, static blocks included: a class whose static block has side effects is a plain
dump problem of its own (0205 follow-ups), as it is for node.
## D2882: `$djs.source(name, hash, params)`: a dump names the codemode body it continues
date: 2026-11-05 · by: pjr · issue: 0207
D2834's `source(ref)`. A continuation of a codemode body names its origin as
`$djs.source(name, hash, params)` instead of a second copy of the body's text. `name` is
the script's file name. `hash` is FNV-1a over its text, parameter names and compile
options (`vm::script_hash`). `params` are the body's parameter names. A djs host compiles
the script `name` it ran, as it ran it, and `Vm::source_function` checks the hash; the
result is the body function, which origin tracking reads as it read the original (a
codemode body's map is its wrapper's, as for its frames). The Sandbox computes the same
hash in JavaScript (`scriptHash`) and compiles with `vm_compile_file`, so the shipped
modules carry no code for it. A host without the script gets
a clear error when the dump loads. The Sandbox knows the scripts its process ran and
takes others from its `sources` option; the CLI reads the file at that path again. node
returns any function, since only dumping again reads the origin. `DumpOptions::source_refs`
(wasm `vm_set_dump_source_refs`, the Sandbox's `inlineSources`) writes the text instead;
the CLI's REPL does so, because its inputs are gone in another process. Only codemode
bodies are named this way: a function inside a script (pi's and the difftest's
`async (tools, console) => …` in a Script) is still written from its text. The sleep
example's dump loses its second copy of the script (2,321 → ~1.9 KB).
## D2951: Objects the dump runtime made are written as its helper calls again
date: 2026-11-05 · by: pjd · issue: 0209
Removes D2903's cost. A run that loaded a plain dump holds objects `$djs.fromSync` and
`$djs.closeThrow` made for a `for await` over a sync iterator: the iterator `{ next,
return, throw }`, the handler, and a step in flight (a `then` on the awaited value whose
handlers resolve the step's own promise). The exporter recognizes them by their closures'
text, which is the runtime's (one table, dump_runtime.rs; a test checks the copies), and
by their captured variables, and writes what they stand for as the engine's equivalents
are written: `$djs.fromSync(it, next)`, `$djs.closeThrow(it)`, and the step's promise as
`then.call(w, (v) => ({ value: v, done }), $djs.closeThrow(it) | undefined)` (the
derived promise of the runtime's own `then` is dropped: nothing else holds it). A re-dump
of such a loop is the size of the first dump again (−1.7 KB per link). A user function
with the runtime's exact text and captures would be read the same way; that is the
price of recognizing by text, and such a function would behave the same.

## D2950: Plain dumps write class text with its static code blanked
date: 2026-11-05 · by: pjd · issue: 0209
A plain dump makes a class by evaluating its text, which ran the class's static blocks
and static field initializers a second time while the dump loaded ("log is not
initialized", or a push done twice); format 2 skipped them in its loader (D2580), which
0205 deletes. Their effects are in the dumped heap, and the exporter writes them after
the class anyway (static values as patches in key order, members deleted since by
`$djs.keep`). So the class text a plain dump writes has the bodies of its static blocks
and the ` = initializer` of its static fields replaced by spaces, newlines and offsets
kept, like the TypeScript blanking of D2879: a simple, written-down rule, no rewriting of
class text into something else. The compiler records the spans while parsing the class's
text (`compiler::class_spans`); only the outermost class's, since a nested class's static
code runs when its own text runs. A static private field or auto-accessor keeps a literal
initializer (only the class's code can set it, D2883); with any other initializer the
class is refused. Cost: `Function.prototype.toString` of such a class after a load shows
the blanks. Rejected: deleting the members from the text (offsets move and the text
differs more), `static n = $dump.vN` (the values must exist before the class, which fails
on cycles and for classes placed in a continuation), refusing classes with static code.
Computed keys, which also run at evaluation, are refused unless literal or a well-known
symbol, as before.

## D2942: The plain-dump fuzzer's second round generates wider values and TypeScript bodies
date: 2026-11-05 · by: pjz · issue: 0211
gen.mjs's programs now also hold Maps, Sets, Dates, RegExps (with `lastIndex` state),
typed arrays (views on part of a buffer), Errors with `cause` and own properties, symbol
keys, strings with escapes, line separators and astral characters, JSON round trips, class
accessors, static fields and static blocks with side effects; every third seed is a
TypeScript body (annotations, interfaces, generics, `as`/`satisfies`/`!`, parameter
properties, sometimes an enum). What a program returns shows those values' state (a Map's
entries, a RegExp's `lastIndex`). `--no-statics` leaves the static members out; `--known
FILE` folds reported signatures into one line. Round-1 seeds give other programs now.
## D2886: `$djs.source(name, hash, params, 1)`: the one function a host's wrapper script makes
date: 2026-11-05 · by: pjr · issue: 0207
D2882 extended to pi's and the difftest's shape. Their codemode function is an arrow that
a host-made wrapper script yields: pi's async body `return (async (tools, console) =>
{…})`, the difftest's Script `(async (tools, console) => {…})`. The exporter names such
a function `$djs.source(name, hash, params, 1)` when the script is that function and
nothing else (only `(`, `return (` before it, `)` after it). Evaluating the wrapper then
makes the function and runs none of the run's code. The host evaluates its wrapper again
before the dump runs and returns that function: pi's adapter from the session's code, the
difftest from the case. node returns any function. A dump's TypeScript is written as
JavaScript only if every TypeScript script whose functions it writes or names is only
types (D2879): a dump that names a script holds only part of it, and an enum outside that
part still changes what the part means. pi suite (next mode, a dump at every idle point,
106 dumps): median 824 → 782 B raw, largest 1,253 → 1,095 B, total 91.1 → 81.4 KB; gzip
median 450 → 448 B, total 47.4 → 47.0 KB.

## D2970: A host function is identified by a host-chosen string, one object per identity
date: 2026-11-05 · by: hb · issue: 0213
What identifies a host function is a string its host chooses (`tools.read`, `console.log`,
`@sdk/our-stuff#myTool`, pi's `call`), not where scripts find it: `DumpOptions::host_fn_name`
returns it, open calls (`$djs.resume(id, name, ...)`), `run.calls`, journals and `HostCall`
carry it. The engine treats it as opaque text; the documented convention is
`specifier#export` for functions that imports will provide and dotted names for
namespace members. A host makes one function object per identity per VM and binds that
object wherever it appears (a global, a namespace member, a body parameter, later an
import), so `===` holds between bindings and across a dump and load (the Sandbox keeps a
handle per id; the CLI's tools live in its `tools` Proxy's table). Bindings refer to
identities; none is privileged: `tools` and `console` are bindings some hosts choose.
Rejected: identities as global paths (what dumps assumed until now), which tied a host
function to a global binding and ruled out a bare `myTool` or an import.

## D2971: Dumps refer to host values by `$djs.host(name)`, never by where they are bound
date: 2026-11-05 · by: hb · issue: 0213
A plain dump wrote a host function it held as its name *as an expression* (`tools.read`,
evaluated while the dump loads), and a host global's value and its members as their
global paths (`tools`, `$dump.a3_0 = tools;` for a body's parameter): so a dump only
loaded where those globals existed, pi had to smuggle its `bridge`/`call` into `$djs`
(D2893), and a captured `tools` holding the host's global was left to the global (D2618's
special case: `let tools = tools` would be a TDZ error). Now `$djs.host(name)` is a hook
beside `seed`, `resume` and `source` (a plain function, D2834) that returns the host value
`name`: a host function is written as `$djs.host(identity)` wherever it is reached; a
*host root* (a value the host made and names: namespace objects, pi's `tools` Proxy) as
`$djs.host(name)`, and its own data properties two levels deep as reads on it
(`$djs.host("tools").list`), so the host resolves root names only. Roots are
`DumpOptions::host_roots` (wasm: a sixth staging line of `vm_dump`, ABI 5) and, as
before, the globals the host lists in `DumpOptions::host`, each named by its global's
name; those globals are still never dumped (D2618's shadowing rule stands). The
identifier check on host function names and its refusal are gone, and so is the special
case for captured host globals: such a variable is `let tools = $djs.host("tools")`. A
djs host answers with the object it bound (the Sandbox: its namespace objects and
function handles; the CLI: its tools table, else the global of the path; pi: the
prelude's `bridge`/`call` and its globals read before the dump runs); node loaders add
`host` to `$djs`. Cost: +0.7% raw, +0.5% gzip over the dump-size corpus (a reference is
`$djs.host("print")` instead of `print`).

## D2973: pi's next mode names its host functions `bridge` and `call`
date: 2026-11-05 · by: hb · issue: 0213
Supersedes D2893. With `$djs.host` (D2971) a host function need not be a global, so the
adapter's identities are plainly `bridge` and `call` (`$djs.resume(id, "call", args,
{"id": n})`), and `$djs.host("call")` returns the very function the prelude was given
(the adapter keeps both handles in `next` mode), where D2893's `$djs.call` was a second
function object. `$djs` no longer carries them while a dump loads.
## D2887: a `yield*` awaiting its inner `return()`: K primed, the request delivered again
date: 2026-11-05 · by: pjr · issue: 0203
An async generator whose `return()` reached an inner iterator through `yield*` awaits that
`return()`'s promise. What comes next depends on the result: if it is done, the generator
returns (through its `finally` blocks), and if not, it yields and the delegation goes on.
Only a `return` request can make K's `yield*` return from the middle of an expression. So
the dump primes K to the delegation's yield, as for a generator suspended there
(`$dump.a.next()`), and delivers the original's in-flight request again as what it was
(`$dump.p = $dump.a.return(v)`). The wrapper `$djs_Y(it, next, 2, promise)` hands out
its placeholder first, and its first `return` gives the awaited promise. K's `yield*`
then awaits it, checks it is an object, and returns or yields as the original does.

## D2972: Hosts bind host functions explicitly: globals, namespaces and body bindings
date: 2026-11-05 · by: hb · issue: 0213
Where scripts find a host function is the host's choice, separate from its identity
(D2970). The Sandbox takes `functions`/`asyncFunctions` keyed by identity and two binding
tables: `globals` (global name → identity of a function or namespace) and `bindings`
(parameter name → identity, given to every codemode body after the run's `params`): a
script that calls `myTool("x")` finds it as a parameter, the stand-in for `import {
myTool } from "@sdk/our-stuff"`. An identity `ns.member` of two identifiers makes the
function a member of the namespace object `ns`, itself a value with identity `ns` that
can be bound anywhere, also as a body binding. Without `globals` one written rule
applies: `console`, every namespace and every function whose identity is an identifier
are globals of their own names (`tools.read` makes the global `tools`); `defineFunction`
binds the same way. The CLI binds `tools` and `console` as a body's parameters
(`BODY_PARAMS`) and `--bind NAME=IDENTITY` adds one; its answers files are keyed by
identity (`"tools.read"`), with no bare-name fallback (D024). The docs runner's bodies
have no parameters. Imports later are one more binding: a host resolver maps (specifier,
export) to an identity, by default `specifier#export`, and the import binding holds that
identity's object; dumps already name it by identity (D2971). Per-run bindings are left
out until a host needs them.
## D2913: A continuation's record makes its layout again: `{f, at, h, r}`
date: 2026-11-05 · by: pjs · issue: 0207
A continuation `K`'s origin record no longer holds its layout (`segs`, `scaf`, `cmap`: 12.5 %
of the v2 dumps' bytes). Origin tracking needs only where `K` copies `f`'s text, where `K`'s
own constructs are and what stands for `f`'s temporaries; generated text in between only
shifts offsets. So the record holds what makes the layout again from `f`: the site `at`; the
tape `r`, every decision the writer took on a value that shapes `K` (an operand or loop
`next` read again, a loop without `return`, a temporary's literal written in place, a
pattern's iterator done, a completion's kind, a `using` scope's resources and state), in
order, without trailing zeros, a number whose bits they are when all are yes/no; and the
holes `h`, `[piece, length, ...]`. `K` is written as pieces (generated runs, copies of `f`,
marks, in-place literals, a run between any two others). The writer writes `K`, then again
from the canonical activation (the site's slot names, every value a placeholder, a dump
that makes nothing later) replaying the tape: the pieces must be the same but for the texts
of runs and literals, else the dump is refused (a decision not on the tape: a bug, no
fallback). A differing text needs a hole unless it reads as the canonical one with a token
(a name, a reference, a number, a string without escapes) for each placeholder. A re-dump
compiles `f`'s script again (as before), writes the canonical `K` with the tape, checks
its fixed text and copies against `K`'s own text (a TypeScript original's types are spaces
there, D2879), takes the holes' and tokens' texts from it and so has the writer's pieces,
whose layout it computes as the writer did. String temporaries are constants of `K`, not
literals (a re-dump reads literals back as values and makes no strings); a run of `K`'s
text that goes on as `f`'s after a copy extends the copy by the longest common prefix.
Score: v2 471,916 → 424,241 raw (gzip 177,258 → 153,023), records 59 → 10.6 KB; sleep
1,247 → 1,093 (648 → 563); corpus 7,286,070 → 7,108,975. Cost: the writer generates each
continuation twice (the corpus's first dumps: about 5-10 % more dump time, mostly the
generator's liveness rendering), each re-dump once per continuation activation.

## D2914: Literal operands, object literals, left operands and substitutions are read again
date: 2026-11-05 · by: pjs · issue: 0207
D2905's rule (an operand evaluated before the site is written as the original's text where
that gives the saved value) now also takes literals (a string without escapes, a decimal
number, `true`, `false`, `null`), which give their value again, and object literals: one
with the site in a property's value (its own key a plain one) is written as the original's
text where every property before it has a plain key (a name or a string, no index, no
`__proto__`, no duplicate) and a literal, or a name that holds the object's value of that
key now, and the object has no other own keys. Nothing sees the object before the literal
is complete, so making it again is the original's evaluation. Elsewhere the object made so
far stays a constant with the rest defined on it (`$djs_D`). The operands of K's own
`$djs_R`/`$djs_C` call stay constants: origin tracking finds the original's temporaries in
K's call only where K's call is the original's. Likewise a binary operator's left operand (a
literal or a name holding the value) and a template's substitution before the site (a name
holding that string). Which construct a temporary belongs to is decided on the site's path:
nested calls and operators start where their callee or left operand does
(`f(a)(await b)`, `1 + (await x) + (await y)`), so `call.callee@N` or `binary.left@N` is
read again only for the node whose arguments or right operand hold the site. Corpus
7,243,739 → 7,216,900 raw (313 `$djs_D` chains, mostly `text({ path, content: await … })`).

## D2915: Positional origin records; a generator's prototype is its function's
date: 2026-11-05 · by: pjs · issue: 0207
`$djs.k(K, f, at, r, h, proto)` replaces `$djs.k(K, { f, at, h, r }, proto)` (0216's queue
item 7): the site an await's number (`2`; other sites by name, `"yield#1"`), the tape and
holes as in D2913, trailing arguments at their defaults (`1`, `0`, `[]`, none) left out;
`K.$djs` is the array `[f, at, r, h]`. A generator continuation's `proto` is written only
where the generator's prototype is not `f.prototype`; otherwise the runtime gives `K`
`f.prototype`. That fixes 0216's finding: a resumed generator's prototype was a fresh
`{ __proto__: %GeneratorPrototype% }`, not its function's `prototype`
(`Object.getPrototypeOf(g) === counter.prototype` was false after a load). The same bug in
general: a function's `prototype` as its definition makes it, held
elsewhere (a frame's `const p = F.prototype`, a generator's prototype), was written as an
object of its own; it is now `F.prototype`, read from the function the dump writes (the
function written first). An unstarted generator whose prototype is not its function's
default one (a `prototype` replaced or changed, which the dump sets after the function) is
made with `Object.setPrototypeOf(Reflect.apply(f, …), proto)`. The golden dumps keep their
object records: they still run (the record is not read on a load), D2619. Corpus case
language/generator-prototype.js.
## D2888: The dump's check compiles the plain text a host loads; `DumpOptions::check`
date: 2026-11-06 · by: pjr · issue: 0215
The checker (0168) compares every function the dump writes with the original. A captured
name must still be captured where the dump writes the function, and a global it reads
must stay a global that exists after the load. It compiled a skeleton of the exporter's
intermediate lines, which are format 2's shape and which no host loads. The plain writer
then moved those functions around (merged scopes, continuation blocks, liveness), so
the check verified the writer's input, not its result. It now compiles the plain text
itself, with the lines that write only data left out as before. The cost is the same
(~47% of a corpus dump, ~34% of a re-dump) and it checks what loads. A function the
plain text does not contain is one the writer left out, which no live binding reads
(D2911), or one it names by reference (D2882): it is not checked. `DumpOptions::check`
(default true everywhere) turns the check off. Whether production hosts do is the
owner's call.

## D2889: The dump's check resolves names without generating code
date: 2026-11-06 · by: pjr · issue: 0215
The check (D2888) needs only every function's free names: what it captures and the
globals it, or a function inside it, reads or writes. `compiler::free_names` parses the
text and runs the first step of resolution, the classification of every name reference.
It skips storage assignment, the second emit and the templates, which made up half of
the full compile. Over the corpus and the dump tests its names are identical to those
read from the compiled code. One difference: a class's text also counts the globals of
its field initializers, which compile to functions beside its constructor. That can only
accept a captured variable that a root of the same name holds, which the class's own
code would read in the same way. Corpus dump 16.3k -> 13.1k M instructions, re-dump
22.3k -> 19.2k M. The check is now ~35% of a dump, nearly all of it parsing.
## D2916: The host deletes `$djs` and `$dump`; the completion value is the last statement
date: 2026-11-05 · by: pjs · issue: 0207
A plain dump no longer ends in `try { (handle); } finally { delete globalThis.$dump; delete
globalThis.$djs; }` (D2890's end; 0216's queue item 3, 13 % of a pi dump): its last
statement is the completion value the host asked for (`$dump.a6;`, `({ __proto__: null,
… });` for several handles, `void 0;` without handles, so that the value of the statement
before is none),
and the host deletes the globals `$djs` and `$dump` once the Script has run (stopped at an
idle point or ended, also when it threw). djs hosts call `Vm::end_dump_load()` (the CLI, the
difftest, the core tests); the Sandbox and pi's adapter run `delete globalThis.$djs; delete
globalThis.$dump` (the module has no delete export, and the size budget no room for one);
node hosts (scripts/plain-node.mjs, the sleep-dump example, js tests) delete them
themselves. `$dump` is the dump's own holder until single-use values go where they are
used (0216's rule 3); the host deleting it is the one interface change that needs.

## D2917: Loops go on as written: builtin iterators as they are, `while` and `for` after the rest
date: 2026-11-05 · by: pjs · issue: 0207
0216's queue item 5, extending D2908. An array iterator ("values") over an array nothing
else holds (dense, ordinary, no other properties) is written as a new one over the elements
left, `["second"].values()`, instead of `$djs.advance(Array.prototype.values.call([…]),
n)`. A loop over a saved iterator whose `next` is read again and whose `[Symbol.iterator]`
is %IteratorPrototype%'s (through frozen objects: a builtin iterator, its own iterable) goes
on over the iterator itself, `for (const x of $djs_i1)`, without `$djs.I` (a decision on the
tape, `for-of.direct@N`); origin tracking reads such a loop's iterator and `next` from K's
own loop. A loop scaffold's constants whose values the dump has before K runs (none the
callback makes, no factory reference) are parameters of K, as the body's bindings (D2909).
A `while`, `do`-`while` or C-style `for` around the site whose body has no `break` or
`continue` is the rest of the iteration, then the loop as written (`while (test) body`;
`{ let i = v; rest; for (update; test; update) body }`), where for a `for` with `let` no
closure in the loop's text may capture the binding (one binding then stands for every
later iteration's); otherwise the placeholder scaffold stays.

## D2918: Assignments around the site are written as the original's
date: 2026-11-05 · by: pjs · issue: 0207
D2914 for assignments with the site on their right side: the target is read again where
it is a name, or a member whose object is a name holding the saved object (not a proxy)
and whose computed key is a literal or a name holding the saved string or number; a
compound assignment's old value where the target holds it now (a name's binding, the
object's own data property). Then K writes the assignment's text (`s += await …`,
`o.p = await …`, `o[k] = await …`, `o.r += await …`) instead of `($djs_t0.r = 1 + await
…)` over constants. Logical assignments and `super` targets keep their form.

## D2919: String temporaries are literals again; a number substitution is read again
date: 2026-11-05 · by: pjs · issue: 0207
D2913 made string temporaries constants of K (a re-dump read literals back as values and
made no strings). A re-dump now finds a string literal of K's text as one of K's own
template constants, or, where K uses it as a key, as the interned atom (without
allocating); so a string temporary is a literal in place again, except an index's string
(`"0"`, an integer key with no string of its own). A template's substitution before the
site whose name now holds a number whose string is the saved part is read again too
(D2914).

## D3000: A placeholder loop's first iteration is a scope of its own for liveness
date: 2026-11-05 · by: pjs · issue: 0207
0216's item 13: the bindings a for-of placeholder scaffold declares for the rest of the
current iteration (`const x = 2;`) are pruned as those of a block (D2911): within the first
iteration's block, not against the whole of K, whose later iterations declare the name
again from the loop's head (`const x = $djs_v1`).

## D2990: Lazy function compilation, a `lazy` feature on in native and dump builds
date: 2026-11-06 · by: lazy · issue: 0218
A unit's compile still parses all of its source with phase-1 emission and classifies every
name, so every early error is reported at load and the enclosing functions' code is what
it was. A lazy function (a function, arrow, method, getter or setter written in source,
not called where it is defined, not owned by a class constructor, without legacy
parameter decorators, and with no name resolved through TypeScript declarations in its
subtree) skips phase 2: it gets a lazy template whose code is the one instruction `lazy`,
with its real flags, name, length, parameters and capture descriptors. At the first call
`lazy` parses the function's text again on its own (`compiler::compile_lazy`), keeps the
result in the template's `IC` field, switches the closure to it and continues in the same
frame. No call path changes. Measured with a prototype (work/research/0218, native):
typescript.js compiles 40% faster, the golden dumps 26%, the dump runtime 35%, and
compiled code on typescript.js's heap shrinks 36%. Codemode scripts gain only 10% of a
~40 µs compile and parse again every callback that runs, so the shipped main and slim
modules leave it out: the code is behind the cargo feature `lazy` (default in djs-core
natively, turned on by djs-wasm's `dump`), and it is image-neutral (D2267). A build
without it treats `lazy` as unsupported bytecode; images do not cross modules (D2560).
`VmOptions::lazy_functions` turns it on per VM (off by default until the dumper reads
lazy templates, 0218 batch 2). `Vm::seal` compiles every lazy template first, so a sealed
image (pi's prelude) never compiles at run time. Shipping it in main later would be a size
trade for the owner.

## D2991: Eager and lazy compiles give a function the same code
date: 2026-11-06 · by: lazy · issue: 0218
Suspension maps come from another compile of the same source (D2563), so a function's
code must not depend on whether it was compiled with its unit or at its first call. The
unit-wide state that the code depends on was the shared constant pool (D2446). With the
`lazy` feature each lazy-eligible function now gets a pool of its own, shared with the
functions inside it that are not eligible, in eager compiles too; eligibility looks only
at a function's own text and its parent's (a call counts only if its tokens end inside
the parent), so the compile of an enclosing lazy function decides the same. A lazily
compiled function's positions are relative to its start; `resolve::Unit` adds the offset
and the line and column back. `vm::lazy_tests` compares every lazily compiled function of
the corpus and the golden dumps with its eager template (code, constants, line table,
names, sizes).

## D2992: A Script keeps its source as UTF-8 bytes
date: 2026-11-06 · by: lazy · issue: 0218
`Script::SOURCE` was a string (Latin-1 or UTF-16) while every template's source span is
in UTF-8 bytes, so each reader converted the whole source: `Function.prototype.toString`
per call, and a lazy function's first compile (D2990) would have been O(script). The
Script now holds the bytes the compiler read, in a `Bytes` cell (`template::script_source`,
`script_text`): a lazy compile copies only its function's span and `toString` converts
only the function's text. Same size for ASCII. COMPAT_VERSION 3.

## D2993: The dump module compiles lazily; the dumper reads lazy templates
date: 2026-11-06 · by: lazy · issue: 0218
djs-wasm's `dump` feature sets `VmOptions::lazy_functions` for every VM (pi's dumping path
runs that module); the native library default stays eager. The dumper reads templates of
closures that may never have run: for a lazy template that ran it reads the compiled one
(its `IC`), and for one that never ran it takes the globals the function and the ones
inside it read from its script's `compiler::free_names` (D2889), parsed once per script
(`Dumper::lazy_globals`); capture names are on every lazy template. A dump of a lazy VM
is the same text as of an eager one (the corpus's plain dumps differ only in the clock).
The gate's plain dumps leg runs `--lazy`, like the stressed difftest and test262 (with an
eager test262 run beside it). A plain dump's own load stays eager (`compile_with_maps`)
until 0218's batch 3.
## D3001: Values used once are written where they are used
date: 2026-11-05 · by: pjs · issue: 0207
0216's queue item 2 (rules 3–4). The writer counts the `$dump.X` references of everything
it writes from (the lines but an activation's and a factory's format 2 line, the
activations' fields and slots but the temporaries K reads again, the factories, the
handles); a value named once besides its own line is written in place of that reference
and its line left out, where that keeps the dump's order:
- a pure value (a literal, a function, a builtin iterator, `$djs.host`/`source`/`bytes`/
  `advance`, `new Map/Set/Date`, reading no name but builtins) may go to any later point of
  the load: a top-level line, a continuation's text, a callback; an anonymous function not
  where a name would be inferred (an initializer, a property's value);
- an effectful one (a reopened host call, an activation, a reaction, …) only to the first
  thing its use's line evaluates with nothing effectful since its own line: a
  continuation's site operand where K is called on its line, is async and does nothing
  before its site (no code of the dump's in its blocks, no callback), the first argument of
  a builtin's call (`Promise.prototype.then.call($dump.a3, …)`), the Script's completion
  value. Host hooks (`$djs.host`, `source`) count as pure: hosts must not make one depend
  on another's order (the CLI's `host` now knows the tools of its answers file).
In a continuation's text a value may read no name K binds there (its parameters and the
declarations on the site's path), except names its own text binds (a continuation written
in place: its top-level names); `this`, `arguments` never. A slot value that would read a
name keeps its reference (it would be read before K, `pre`). Liveness and K's helper
captures do not look into the site operand (`P::Val`). The origin's function of a record is
written in place (no `$dump.FaN`); the holder `globalThis.$dump = {}` is left out where
nothing reads it. A hook the dump calls inside a continuation throws into that activation,
not the Script: hosts fail the load on a hook they could not serve (Sandbox, pi's adapter,
the CLI). A value read again by name after it was written in place is a refusal (a bug).
## D3020: Dumps take the base's builtin names from the build, not from a walk of the base
date: 2026-11-06 · by: pjr · issue: 0215
A dump writes a builtin by its path (`Array.prototype.push`) and leaves out a global the base
already has. For that it walked the whole base image (`registry::reachable_names`, every
builtin object's properties) and copied the base into a heap to compare the global object
with. This happened once per VM, at its first dump. A host that dumps each VM once pays it
every time: pi with a dump at every idle point loads every dump into a new instance and
dumps that once. The walk was ~40% of a small dump natively. In wasm the host also had to
hand the base over first (`vm_dump` returned 2), and the instance checked it by hashing.
djs-prelude's `names` feature now emits the walk's result when it builds the base:
`BASE_NAMES` (cell, path) and `BASE_GLOBALS` (the global object's properties as value bits
and flags). Hosts that dump pass them as `DumpOptions::base_names`, which makes `base`
unnecessary. The checker's scratch heap is then a fresh one (compiling needs only atoms).
The dump module carries the tables (+26 KB raw, about 1,600 builtins) and no longer asks
the host for the base, so `vm_dump` returns only 0 or 1. The output is byte-identical over
the corpus. Corpus re-dump (a loaded VM's first dump) 19.2k -> 11.6k M instructions. In
wasm, in pi's adapter, a dump is ~1.24 -> ~0.3 ms at the median.

## D2994: A lazy VM loads a plain dump lazily; maps of other functions on demand
date: 2026-11-06 · by: lazy · issue: 0218
`Vm::compile` of a plain dump in a lazy VM compiles it lazily and keeps the suspension maps
of the code the load compiled (`compiler::compile_lazy_with_maps`, every map, those without
sites too) as a partial entry of the VM's dump cache, with the globals of each lazy
function (D2993). When a dump needs the map of a function the entry lacks (an activation's
function compiled at a first call, an origin that never ran, the function that evaluated a
class's computed keys), `Dumper::script_maps` finds the innermost lazy template around it
(`LazyIndex`, one walk of the heap per dump) and compiles that function alone with
recording, every function inside it eager (`compiler::lazy_function_maps`); by D2991 the
maps equal an eager compile's (`vm::lazy_tests::lazy_maps_are_eager_maps` checks it over
the corpus and the golden dumps). Without such a template it compiles the whole script
again, as before. A lazily compiled codemode script keeps an entry too (globals only, no
maps). Measured (native, instructions, `scripts/dump-costs/instructions.sh`, with D2995):
corpus load 9,465 -> 8,420 M, re-dump 14,394 -> 14,572 M (load + re-dump -3.6%); records,
log-5mb and binary within 1%; pi's sessions (wasm, `pi-sessions.ts`) load and dump about
4-5% faster.

## D2995: Functions that run at once stay eager: call arguments, parenthesized, async
date: 2026-11-06 · by: lazy · issue: 0218
A lazy function that runs is parsed twice. Besides IIFEs (D2990), a function expression
or arrow that is a call's argument (`items.map(x => x.id)`) or parenthesized (`(function
() {})`, the convention other engines use for "runs soon") stays eager, and so does every
async function and generator: in dumps those are the continuations and origins whose
suspension maps a re-dump needs anyway, so compiling them lazily only moved their compile
(and doubled it for continuations). The rules look at the function and its parent's text
only (D2991). The dump runtime compiles eagerly in a lazy VM: a host passes the helpers its
dump names, and the load calls them. With these the corpus's codemode runs cost 4% more
instructions lazily than eagerly (42% without the argument rule), and typescript.js still
compiles in about half the time.
## D3021: Array.fromAsync in flight is written as the runtime's `fromAsync`
date: 2026-11-06 · by: pjr · issue: 0215
Array.fromAsync runs as a state machine over a Record (builtins/fromasync.ts). Two builtin
closures resume it from whatever its await settles. Plain dumps refused them ("an
internal function of a builtin"). The dump runtime now has `fromAsync(p, mapfn, thisArg,
it, next, A, k, phase, items, len, error)`: the same machine in JavaScript, resumed in
the phase it waits in. It attaches its handlers to `p` as the engine's reaction was
attached and returns the result promise. The exporter writes the reaction on the awaited
promise as that call (its promise is the call's result), so the reactions keep their
order. It recognizes the runtime's own handlers by their text and reads the state from
their captures, so a re-dump writes the call again (as for `fromSync`, D2951). An
async-from-sync iterator brings its own `next`. Errors the engine raises, such as "Array
too long" and "iterator must return an object", keep their type and message.

## D3022: Amends D2896: the gate also cleans djs-core and djs-prelude after incremental corruption
date: 2026-11-06 · by: pjr · issue: 0215
D2896's retry cleared the `incremental/` directories of the failing job's target
directory and ran the job again. A crate already built from the bad cache stays
fingerprinted as fresh, so the same link failed on every later run. In 0215 the djs-prelude
build script failed against a corrupt `libdjs_core-*.rlib` in the gate's
`target/wasm-nodump` until the coordinator cleaned it by hand; pjs had hit the same before.
The retry now also runs `cargo clean -p djs-core -p djs-prelude` in that target directory,
for the host and for each target triple it holds (`clean_corrupt_rlibs` in scripts/gate.py).

## D3023: pi's adapter reuses disposed wasm instances, made as new before reuse
date: 2026-11-06 · by: pjr · issue: 0215
Creating an instance was most of a session's start from a cached prelude image (~100 µs:
V8 reserving and mapping its memory). That cost was paid at every load in a dump at every
idle point, and at every start. js/pi/engine.ts keeps up to two instances whose VM was
disposed, each with at most 64 MB of memory, and hands them to the next VM. Before reuse,
an instance is made as new: its memory up to the initial size is copied back from the
module's first instance as it was before any call (static data, the allocator's state, the
stack) and the rest is zeroed. Nothing of the earlier session is left in it, whoever runs
next. The allocator starts again and takes the grown memory as free pages. Renewing a
1.4 MB instance costs ~17 µs. A new instance in pi-sessions: ~103 -> ~32 µs at the median.

## D3024: A waiting frame's `this`, `arguments` and `new.target` that its arrows share are K's
date: 2026-11-06 · by: pjr · issue: 0215
An arrow that reads the `this`, `arguments` or `new.target` of an async function or
generator that is waiting captures it as a box of the frame (a hidden binding). Such
boxes were refused ("a captured `this`, `arguments` or `new.target`"), which refused
every class method doing `items.map(x => this.f(x))` around an await; and an arrow made
before the await that read `arguments` read K's own arguments object after a load (empty)
instead of the original. Now the box is the frame's like any shared variable, so the
arrows are written in K. `this`: K is called with the frame's `this` already, so K's own
`this` is the binding, and no variable is written for it. `new.target`: an async
function or generator is no constructor, so it is undefined in both. `arguments`: the box
is named `$djs_arguments`, K's binding of the saved arguments object (the same object, as
before for K's own text), and the arrows' texts read it under that name: the frame's
function's syntax lists its `arguments` references (its arrows' included, not those of
nested functions), so `fn_source` renames exactly those inside an arrow from the same
script. A box of a call that has returned stays refused (a factory has no `this`).
Corpus: codemode/arrows-share-this-arguments.js.

## D3002: A continuation's site takes its value from its callback; effectful values in a run
date: 2026-11-06 · by: pjs · issue: 0207
0216's queue items 6 and 10, first part. Where a continuation's prefix calls the last part
of its callback right before its site (K async, called on its own line, no code of the
dump's in its blocks below that part's), a value that part makes for the site alone is
made at the site instead (`report.push(await Promise.all([2, …]))`), reading no name K may
bind (any declaration of its function, any slot's name, its `$djs_` helpers). Effectful
values may go to the *run* of a line's first evaluations: the value itself (its receiver,
`$dump.a3.then(f)`), or a builtin call's or an array's leading operands with literals
between (`Promise.all([$dump.p4, $dump.p5])`), all or none, where their lines are the last
effects of the list, in that order. A reaction on a promise of the builtin prototype
without own `then`/`finally` is written as the method call (`p.then(f)`, trailing
`undefined` left out): while the dump loads, builtins are the realm's own.

## D3003: Closures and continuations a continuation's blocks make are written where they are used
date: 2026-11-06 · by: pjs · issue: 0207
0216's queue item 6, second part (factory plumbing). A closure of a merged scope placed in
a block of continuation K (written there as `Object.assign($dump.sN, { key: … })` and read
back as `$dump.sN.key`) whose one use is in K at or below that block (a binding's value,
the callback lines K now holds, the site's operand taken from the callback) is written at
that use instead, the member left out of the block: `var todos = async function todos(path)
{…};`. Only a member written as `key: <function>` (a method keeps its home object), only
where no block between declares a name the function reads; `(0, f)` loses its parentheses
where no name is inferred. A function's creation evaluates nothing, so it may come later
than its block's code. A continuation of a merged scope is called on `$djs.k`'s result
(`$djs.k($dump.sN.$Ka5, f)(…)`, `$djs.k` returns K) and written there where that call is in
K below its block (`$djs.k(async function (path) {…}, todos)("src/d.ts")`). In those texts a
member a binding of K declared at or above holds is named by the binding (`todos`), and
liveness sees the bindings the dump names in the site's operand (`P::Val`'s names). Only
references outside function bodies are rewritten: a continuation's text inside has its
layout (holes) already. A merged factory's holder object is written only where a member is
still written on it. Values and activations nothing names are written as statements, or
left out where they only make a value.
## D3040: A compile cache across VMs that replays the compile
date: 2026-11-06 · by: lazy · issue: 0219
pi loads its session's script (for `$djs.source`) and small fixed snippets again at
every idle point, in a new instance. `Vm::compile_cached` (feature `compile-cache`, on in
djs-core natively and through djs-wasm's `dump`, so only the dump module carries it)
records a compile: a compile only appends cells (allocation never collects) and interns
atoms, so while it runs the heap logs every interning with the arena length before and
after (`Heap::intern_log`), and the blob is the appended cells in order with the
internings between them. An import (another VM of the build, the blob's key: text, file
name, parameters, options, lazy or not) replays it: it copies the cells, interns the same
texts at the same points (the interning's own cells, an atom's string or table growth,
come from the replay), rebases references inside the blob, and renumbers atom ids in the
templates (names, local names, atom immediates, global declaration lists) if this VM
numbered them otherwise. Because the internings happen in the compile's order, the
importing VM ends up byte for byte as if it had compiled the text, also from another
state; string hashes made under another hash key are cleared. A blob that does not fit
compiles over. Tests: `vm::compile_cache::tests` compares heaps over the corpus, from the
same and from another state, before and after running; the difftest's `--determinism`
compiles each case through a blob from a copy of the VM and requires the same heap.
pi's `Engine.compile` keeps blobs per module (64 entries, 8 MB). Import costs about a
quarter of a compile (a 2.6 KB codemode script: 8 µs for 32 µs native).

## D3025: A block's placed code waits for the late binding it reads; a re-dumped method's home is its function's
date: 2026-11-07 · by: pjr · issue: 0215
Two subclass bugs from the plain-dump fuzzer (pjz). First: a block of K writes the dump's
code for that block (classes, members of a scope the frame shares) before its "late"
bindings, those whose values read a factory's members (`let D3 = ((C2) => class D3 extends
C2 {…})($dump.s9.C2)`), so that code made the members first. A member that names such a
binding (`Object.assign($dump.s8, { D3 })`) then ran in its TDZ and the load threw. Now a
placed line whose own evaluation reads a late binding (its names outside function and class
bodies, keys left out) goes right after that binding's line, and the placed lines after it
go with it (their order stays). It doesn't move where a late binding it would follow reads a
factory that it or a later placed line fills. Every other dump stays byte for byte as it was.
Second: a subclass's async method that awaited `super.fetch(...)` was dumped with K
resuming that call, so K uses no `super` and has no home object. At the next dump origin
tracking found no `<home>` in K's frame for the method's own, and no super prototype for the
regenerated K. Both now come from the origin function's home object (the value its
`<home>` local holds, D2871's form gives the new K that home's prototype).

## D3026: Late bindings and subclasses wait for the block's own declarations; keys keep lone surrogates
date: 2026-11-07 · by: pjr · issue: 0215
Found by the fuzz regress after D3025 (pjz's s14-s11245 program, and fresh seeds 82489,
832780 and 668041). First: a binding of K whose value reads a name that the block's placed code
declares (a base class's local, `((C2) => class D3 extends C2 {…})($c30)` after `let $c30 =
class C2 {…}`) was early, so it ran before that declaration. Now such a binding is late,
among the late ones in declaration order. Second: D3025's check of whether a late binding
waits for a placed line was per factory, which kept `Object.assign($dump.s8, { D3 })` ahead
of `D3` whenever another late binding read any member of `s8`. It's per member now: a value
waits for a line that fills `F` only if it reads `$dump.F` itself or a member `K` the line
names. A placed line also moves no further than the lines before it unless that check holds
there too; the function's own name in `{ ag7: async function* ag7() {…} }` had moved it
behind the binding that reads what it makes. Only dumps that failed at load change, plus two
corpus cases where a placed line now follows a binding it doesn't need (both run the same).
Third (seed 668041, also on main): a class statement of a shared scope went where the
variables it reads are declared, but a subclass reads its base's local
(`((C27) => class D28 extends C27 {…})($c25)`), so it could land in a block above the base
(K's top while the base, reading a loop body's binding, was in the loop's block). A class
statement that reads another one's local now goes no higher than that one (the innermost of
their places, to a fixed point). Fourth: an object key with a lone surrogate ("\ud800" + "9") was quoted from its text, where
it is U+FFFD. Quoted keys (literals, members, `Object.defineProperty`, `Symbol.for` keys)
now come from the heap string itself, as values always did; well-formed keys quote byte for
byte as before.

## D3027: An object literal's method that uses `super` is made where its home is
date: 2026-11-07 · by: pjr · issue: 0215
A method's `super` reads its home object's prototype, and the home is fixed when the method
is made, so the dump can't write such a method on its own. That refused every object
literal's async method that awaited before or after using `super`: its frame names the
method, so it had two references, and only a method with one was written in its literal.
Now a method whose home is the literal holding it goes in that literal whenever the
literal has a location, whatever else names it (its frame, K's call, a binding like
`const alias = o.m`); the others read it at `holder.key`. A binding whose value is such a
method is written after its home's binding (roots aren't in declaration order). And a
method that is a member of a scope the dump makes (it reads the scope's variables, as a
codemode method reading `tools` does) had silently got the factory's own literal as its home:
`super.label` read `Object.prototype` after a load. It is now written as `key: ({ __proto__:
P, async key(…) {…} }).key` with P its home's prototype, as a continuation's K is made
(D2871). `super` reads the same prototype, though a later `Object.setPrototypeOf` on the
original home isn't followed, as with D2871. Where P can't be written yet, the dump refuses.
Corpus codemode/object-method-super.js; tests/dump/v2/supers.src.js.

## D3150: A block's dump code and late bindings go in the order what they read needs
date: 2026-11-07 · by: pjr · issue: 0235
Plain-dump fuzzer seeds 342347 (found with `--lazy`, also without), 344676 and 208872 crashed
at load. All three were D3025/D3026's placement of a block's dump code around K's late
bindings: one `Object.assign($dump.sN, { … })` line can fill a member a late binding
reads (`const f33 = $dump.s8.f33`) and also read a later late binding (`D39`). Cutting the
late list once can't satisfy both, and a function's own name counted as a read (`{ ag42:
async function* ag42() {…} }`) blocked the move. Now the block's lines are ordered by a
small list schedule. Hard edges: a late binding comes after the code line that fills a
factory member it reads (`$dump.F.K`), after the line that declares a name it reads, and
after another late binding whose name it reads; code lines keep their order. Soft edge: a
code line comes after the late bindings whose names it reads. Each step takes the first item
in the written order (code, then the late bindings) whose edges hold; when none does, the
soft edges give way, then the order as written. Where nothing constrains them, late
bindings stay in declaration order after the code, as before. In the corpus only one dump
changed (a binding now precedes a line it doesn't need), and it runs the same.

## D3151: A subclass outside its base's factory names the base by its location
date: 2026-11-07 · by: pjr · issue: 0235
Reduced from seed 342347: a class that captures nothing (`class D4 extends C3 {}`), whose
base is a class of a scope a frame shares (written in K's block as `let $c29 = class C3`),
was written as top-level data `((C3) => class D4 extends C3 {})($c29)`. It read the base's
local, which exists only inside K's block. The local is meant for the classes of the
factory being written, but any class rendered meanwhile took it. Now only the group's own
classes use it. Another class names the base by its location (`$dump.s8.C3`), and the plain
writer then makes it after the block code that publishes the base, in K's callback.


## D3152: plain-node reads node's "Invalid time value" as djs's "Date value is NaN"
date: 2026-11-07 · by: pjr · issue: 0237
Plain-dump fuzzer seed 689441 (found with `--lazy`, also without) was "node wrong output": an
invalid date's `toISOString` throws a RangeError in djs, as in qjs, worded "Date value is
NaN", and in node worded "Invalid time value". The dump was right, and djs's plain and dumped
runs agreed. scripts/plain-node.mjs maps node's wording to djs's, as it already does for
other common errors. Corpus codemode/plain-fuzz/date-nan-iso-string.js (fails with the old
mapping); the program is frozen as regress/s0237-s689441.js.

## D3041: Suspension maps travel from one VM's dump to the next VM
date: 2026-11-06 · by: lazy · issue: 0219
A dump compiles the suspension maps of the scripts its activations run in (pjr: ~30% of a
pi `vm_dump`), and pi's next dump runs in a new instance that would compile them again.
The dump cache's entries for texts that are not plain dumps are exported as bytes after a
dump that compiled maps (`Vm::export_dump_maps`, `vm_dump_maps_export`; `compiler::
smap_codec` encodes the maps) and imported into the next VM before its dump
(`vm_dump_maps_import`); the maps are pure data of the text and options, so a dump is the
same text either way. Measured on pi's sessions (wasm, `pi-sessions.ts`, instructions of
the whole process over 300 sessions): -4.0% with both caches (-2.7% from the maps alone,
-0.9% from compiles alone), -6% of the instructions per session; timings on a quiet
machine: dumpSession 143 -> 128 µs, resumeSession 121 -> 116 µs (median).

## D3004: A reopened host call read once waits for its reader
date: 2026-11-06 · by: pjs · issue: 0207
A `$djs.resume(…)` line whose value the dump reads once is not written where the exporter
listed it but right before the first line that reads it (with the deferred calls before
it, in their order): there it is the last effect, so it goes in place (D3001), typically
into the continuation that awaits it (`await $djs.resume(5, "later", [2])` in each element
of a `Promise.all`). Host calls keep their order among themselves (the host's order); what
they move past is the dump's own code, which hands them nothing and makes no host call.
Each is written into the list it was listed in (a callback's lines run after the lines
that follow it outside), at the latest when that list ends.

## D3005: A hole the reader finds by reading the value is left out of the record
date: 2026-11-06 · by: pjs · issue: 0207
A continuation's record lists the holes (D2913) whose length the reader cannot find in K's
text. Before, the reader took a placeholder's text as one token (a name, a number, a string
without escapes); now as a value: a token or an expression read up to the first `,`, `;`,
closing bracket or line end outside brackets, strings, templates, comments and regular
expressions, its spaces only before a name or a bracket (`await $djs.resume(2, "call",
[…], {…})`, `new Map([…])`, `async function (…) {…}`). The writer leaves a hole out only
where this reading gives the value's length in its own text, so a reader scanning the same
text finds the same; anything else keeps its hole. Records with holes: 2,552 → 736 of the
corpus and v2's.

## D3006: Arguments in place outside K, functions movable, values nothing reads dropped
date: 2026-11-06 · by: pjs · issue: 0207
- A continuation's call arguments get the values written in place when its call is
  written (they are evaluated outside K, where K's bindings do not matter); K's generator
  decides on the values as listed.
- A function's text counts as a pure value (its creation reads nothing; where a name it
  reads would be another binding, the reads checks keep it out).
- Where K is async, unhomed and does nothing before its site (no code of the dump's in
  its blocks, no callback), its site operand is worked out before K is written, so that
  the values its K's bindings name there (`$djs.k(…, step)("first")` for a function both
  K's parameter `step` and the inner record's origin) are known to be read once when K's
  arguments are written: the sleep dump 797 → 751 bytes. A value on `$dump` is named by a
  binding only where the original reads the binding after the site or in a loop around it
  (naming it would keep a dead binding); a member of a factory always.
- Before the Script is put together, a top-level `$dump.X = value;` nothing reads any more
  (a parameter liveness left out held it, a reference became a binding's name) is left out
  where it only makes a value (a factory too) and written as a statement otherwise, until
  none is left.

## D2837: The seed is written only when the run used Math.random before the dump
date: 2026-11-05 · by: owner · issue: 0207
`$djs.seed` appears in a dump only if `Math.random` was called before it: an unobserved
generator state is consistent with any seed. The call-id part of the seed line (D2897) stays
wherever ids need it. Tests that compare a resumed run with the uninterrupted one seed
deterministically.

## D2838: pi leaves out a reopened call's state when it equals the call id
date: 2026-11-05 · by: owner · issue: 0207
`$djs.resume(id, name, args)` without the fourth argument means "state = id" in pi's adapter
(114 of 121 calls in the pi suite).

## D2839: Source references keep the script's name and use a 32-bit hash
date: 2026-11-05 · by: owner · issue: 0207
`$djs.source(name, hash, params)`: the name stays (readability; the CLI finds the script by
it), the hash shrinks to 8 hex digits, enough to tell a host's scripts apart.

## D2840: Dumps carry no "use strict"; hosts compile them strict
date: 2026-11-05 · by: owner · issue: 0207
Dumps are loaded by djs hosts only, which compile them as strict code. Test harnesses that run
dumps in node (scripts/plain-node.mjs) prepend the directive themselves.

## D2841: Settled by the owner on 0216's review
date: 2026-11-05 · by: owner · issue: 0216
Generated names and whitespace stay readable (short names save under 0.5% gzip). The dump-time
check stays on in every host (about 0% of a pi dump). Lazy compilation stays out of the shipped
main and slim modules; no per-seed prelude images for deterministic loads. Confirmed: static
code blanked in dumped class text (D2950), host bindings and identities (D2970-D2973), the
deliberate refusals (instances with private fields, for-in over a Proxy, an await after another
resource in a `for (using …)` head). The size ceilings stay ratchets: room is made by shrinking
something else.
## D3007: Host calls go into the callback part that reads them all; a factory's continuation takes its site's value
date: 2026-11-06 · by: pjs · issue: 0207
Extends D3004 and D3001. Deferred host calls are written into a part of a continuation's
callback (which runs later, at K's go point) where every call still waiting is read in that
part (by its items, its activations' operands and slots, or its own callbacks): then none
is left to be written outside before the part runs while coming after one written in it.
A continuation of a factory's scope is called on `$djs.k`'s result on its own line
(D3003), so where its text is written at that call, its site's operand may take an
effectful value as the first thing that line evaluates, as an unhomed one does. A name
read only as an object literal's key (`{ path: … }`) does not keep a closure or
continuation from being written below a block that declares it.

## D3008: How D2837–D2840 are carried out
date: 2026-11-06 · by: pjs · issue: 0207
- `Math.random`'s state record holds the seed's halves too (elements 2–3, set with the
  state by `set_random_seed`/`set_math_random_seed`; `COMPAT_VERSION` 4): `Math.random`
  ran where the state differs from it, and only then does a dump write the state (a draw
  costs nothing more). A host restores a dump's state with `Vm::restore_math_random`
  (wasm `vm_restore_math_random`; one code path with `set_math_random_seed`,
  `Vm::set_math_random`, for the slim module's ceiling), which records the seed as another
  value: a re-dump of a
  loaded run that drew numbers writes the state again.
- `$djs.seed` has three forms: `(lo, hi, next)`, `(lo, hi)` and `(next)` (a second
  argument `undefined`), or is left out. `next` is written only where it is not one past
  the highest reopened call's id; every host now sets the id counter after a load to
  `max(next ?? 0, last + 1)`.
- The source hash is the 64-bit FNV-1a folded to 32 bits (`h ^ h >> 32`), 8 hex digits,
  in `vm::script_hash` and Sandbox's `scriptHash`.
- pi's adapter writes no state for a call whose pi id equals its call id and reads a
  missing state as the call id.
- Without `"use strict"` the dump's first line is `"djs-dump";`. The node harnesses
  (plain-node.mjs, the sleep example, js tests) prepend `"use strict"; ` on that line, so
  line numbers stay the dump's.
## D3050: `debugger;` can be a pause: an await of the host's `debugger`, in dump builds only
date: 2026-11-06 · by: guide · issue: 0220
With the compile option `debugger_awaits` (bit 6 of `CompileOptions::bits`, `vm_compile`'s
`| 16`, the Sandbox option `debugger`), `debugger;` in an async function compiles as the
statement `await debugger();`, `debugger` being the global a host binds (identity
`"debugger"`), and the suspension map records it as that statement with its Await node
over the keyword. A run is then idle at the statement, and the exporter writes the pause as
any open host call (`await $djs.resume(id, "debugger", [])`), with no case of its own; a
later `debugger;` in a continuation's text is copied as written and pauses again when the
dump is loaded with the option. Outside async functions, and without the option, the
statement does nothing. The option is a field of `dump` builds only (cfg-gated, like the
parser branch), so the shipped and slim modules keep their size (only the source hash
constant changes); the Sandbox refuses it for a module without the dumper, since a dump
compiles the script again with the same options. It exists for the docs site, whose live
examples show the dump a run writes at a `debugger;`. Alternatives: rewriting the source
in the page (columns move, the dump shows the rewrite) or a new opcode or intrinsic (the
image descriptor of the dump module would then differ from the shipped one's).

## D3051: The guide cites no decisions, tells no history and names pi on one page
date: 2026-11-06 · by: guide · issue: 0220
Owner's rules for docs/guide, enforced by `djs-docs check` on the pages in SUMMARY.md
(prose and inline code; code blocks exempt): no decision IDs and no links to the decision
log (which leaves the site, with its search entries and the `gen:decisions` region); no
history (a short list of telling phrases: "no longer", "used to be", "originally", "was
replaced", "format 2", ...); pi, as a whole word or a path (`js/pi/`), only on `pi.md`, so
`π` and `pipeline` pass. Generated descriptions (opcodes, intrinsics) drop their decision
citations; `gen:status` names pi only on README.md and `pi.md`. Supersedes D2102's "the ID
only as a citation". New: `gen:dump [N]`, the dump the block above writes at its Nth
`debugger;` pause (D3050), checked like `gen:run` and accepted with `regen --examples`.
While the rewrite runs, pages it has not reached yet are listed in `NOT_YET_REWRITTEN`
(render.rs), exempt from the pi and history rules; the list ends empty.

## D3060: `debugger;` pauses in the CLI: `djs repl --frozen` and `--debugger OUT`
date: 2026-11-06 · by: dbg · issue: 0221
The CLI's dumping modes are its debugger modes: `djs repl --frozen`, and `djs run` /
`djs resume` with `--debugger OUT`, compile inputs, scripts and the dumps they load with
`debugger_awaits` (D3050) and bind the global `debugger` (identity `debugger`). A pause is
then an open host call like a tool's, so `.pending`, `.bt`, `.dump`, `.restore` and
`$djs.resume(id, "debugger", [])` need no case of their own; the CLI tells a pause by the
call's identity. REPL: an input with a `debugger;` outside the functions it defines (a
token scan) runs as a top-level-await input, so it pauses at its own top level; a pause
prints `[paused] #id input N at repl:L:C` with the line, `.continue`/`.c [id]` answers it
in place, `.dump` + `.restore` + `.c` continues from the dump, and `.abort` (Ctrl-C at an
empty prompt) is `vm.abort()`: it drops the paused input and everything else in flight,
since one input cannot be dropped alone (rejecting the pause would run the input's
`catch`/`finally`). `run --debugger OUT` writes the dump at the first new pause and stops
(exit 0, `djs: paused at FILE:L:C, wrote OUT`), parallel to `--dump-after K OUT`;
`resume --debugger OUT2 OUT` stops at the next one, stepping pause by pause. A resumed
run answers the pause its dump reopened at once, and without the flag every later one (it
always binds `debugger`, since a `--body` dump compiles the file again with the options
its hash names, and `$djs.source` tries `debugger_awaits` among them). Without these modes
`debugger;` does nothing.
## D3052: goals.md holds every goal; links to a page's section must exist
date: 2026-11-06 · by: guide · issue: 0220
Owner's rule: every goal the guide refers to or relies on is a section of goals.md, which
says what it means for a user, why djs chose it, and what it costs or rules out. Five are
ranked (cheap data passing, fast startup in wasm, no syntax tree, persistence, suspend
anywhere); four hold under all of them (isolation, determinism, wasm32 first, QuickJS's
behavior). Minor aims are folded in (a small module and heap under fast startup,
TypeScript under no syntax tree) or said in plain words, never called goals. `djs-docs
check` fails on any link to a section a page does not have (`page.md#id` and `#id`,
against the ids `render` gives headings, `render::heading_ids`), on every page.

## D3053: live examples pause at `debugger;` in their normal Run
date: 2026-11-06 · by: guide · issue: 0220
Owner's design for the docs site, replacing the Dump tab of D3050's demo: an example whose
code has a `debugger` keyword runs, when the site's module has the dumper, as a task with
the Sandbox's `debugger` option and frozen builtins (dumps need them; examples without
`debugger;` run as before). At each pause the page suspends the task into a dump and
disposes of the sandbox; Run becomes Continue, which resumes from the dump in a fresh
sandbox (`Sandbox.resume`) to the next pause or the end. The output is one transcript
with a line at each pause (its line when the code has one `debugger;`, the dump's size);
the current dump, with raw and gzip size, is a `<details>` under the output, which the
build fills with the first pause's dump for readers without JavaScript. A dump the writer
refuses is shown with its reasons, and Continue goes on in the same sandbox. An info tip
left of Run explains the pause and links dumps.md. `ts` blocks (static and live) compile
as TypeScript, in `gen:run`/`gen:disasm`/`gen:dump` and in the browser.

## D3054: where a run waits: `Vm::call_site`, `vm_call_site`, `Sandbox.callSite`
date: 2026-11-06 · by: guide · issue: 0220
The live examples' pause lines name the exact position of every pause, as the CLI's
`[paused]` lines do (D3060). `Vm::call_site(call_id)` (djs-core, `dump` feature only)
returns `{file, line, col}` of the innermost suspended activation waiting on the open
call's promise: breadth first over the promise's reactions, through combinator element
closures and capability records, the first Generator found (an `await` awaits the call's
promise itself, so a `debugger;` pause is found at depth one; a pause inside a
`Promise.all` callback is its own activation). The position is that activation's pc
looked up in its function's line table, and the file its Script's name: the host's
filename for the code the run started with, `dump.js` (the loader's name) for code a
dump carries. A run continued from a dump runs the dump's text, so later pauses are
positions in the dump it continued from, not in the original: mapping back through origin
tracking's layouts would need the dumper's normalization, which this does not touch. The
docs site shows them as such ("line 7:3 of dump 1", a link that shows that dump with the
statement marked); every dump of a run stays reachable from the transcript. wasm:
`vm_call_site(vm, id)` writes `line:col:file` to the out buffer (0 = none), in dump
builds only; the shipped and slim modules are unchanged (1,033,538 and 906,238 B before
and after, differing only in the source-hash constant). js: `Sandbox.callSite(id)`
returns `{file, line, column}` or `undefined` (no export, call not open, nothing waits).

## D3070: Measured figures in prose are inline status numbers
date: 2026-11-05 · by: meas · issue: 0223
A measured number in the guide's prose or README.md is written `<!--n KEY FMT-->TEXT<!--/n-->`
and regenerated like a `gen:status` region (D2101): KEY is a repository count djs-docs
computes or a key of `tests/scores/status.tsv`; a missing key fails `check`. So every
figure the guide quotes (dump sizes and times, module sizes, heap bytes per object,
bounds-check cost, startup, pi's benchmark) has a measurement behind it, and a re-recording
updates every page that quotes it. `scripts/status.py` gained the measurements: `size`
(shipped, slim and dump modules, coded base, exports), `pi` (pi's tests in three modes,
js/bench/pi.bench.ts incl. interrupt polls per second), `startup`, `dumps`
(scripts/dump-costs, packaging B), `bounds` (the `unsafe-heap` build against the shipped
one), `heap` (scripts/size/object-bytes.ts, wasm memory), `bench` (ratios from the latest
`bench.sh --record`), `all`. `gen:status recorded` lists all records (pi's on pi.md).
Figures nothing could re-measure were removed from the prose (gc.md's collection-time and
peak ratios, compiler.md's per-function lazy costs). Numbers measured on a branch whose
engine sources equal its base are recorded at the base commit, which survives the rebase
(0223).

## D3090: The shipped module builds core and alloc itself, with panics as bare traps
date: 2026-11-05 · by: bytes · issue: 0225
scripts/wasm-build-env.sh (every build of the shipped, slim, dump and docs modules) adds,
under D2333's RUSTC_BOOTSTRAP=1 and only there: `CARGO_UNSTABLE_BUILD_STD=core,alloc`
(cargo's `-Zbuild-std`), `-Zunstable-options -Cpanic=immediate-abort` and
`-Zshare-generics=no`. With immediate-abort every panic is an `unreachable` instruction:
the call sites lose their arguments (index and length, message pointers) and core's
panic functions, messages and formatting code go away. The release module recorded no
message anyway (D2332, a fixed text), so nothing is lost; js/src/trap.ts reports an
`unreachable` trap without a recorded message as "engine panic (...)". Debug builds and
native builds keep the messages.

`-Zshare-generics=no` is part of it: at opt-level s rustc shares generic instances with
upstream crates, and with core built at s (instead of the precompiled O3 one) djs-core
called core's out-of-line `copy_from_slice_impl::<MaybeUninit<u8>>` 67 times from
`run_hot` (the arena accessors): +40% wasm instructions on tight, arrays and forof.
Building only core at opt-level 3 did not fix it; core+alloc+compiler_builtins at 3 did
but saved only 6.9 KB.

Measured (shipped module, gzip -9): 442,576 → 431,762 (−10.8 KB; code −8.6, data −2.2);
slim 366,126 (ceiling was 378,201). Wasm instructions per workload (scripts/perf/wmeasure.py -n 3,
work/scratch/0225/wm-iasg.txt): tight +1.9%, prop1 +1.6, strings −1.4, arrays +0.4, json
−0.6, json-date +2.3, forof −1.1, calls +0.9, push-pop −1.8, pi-like −3.1, foreach −0.7,
higher-order −2.4. Without build-std the flag cannot be used (the precompiled core has
the unwinding panic paths). Rejected on the way: wasm-opt -Oz/-Os/-O2/-O3 (raw −6%, gzip
+1.1 to +3.5 KB), `-Zfmt-debug=none` (−0.1 KB), core's `optimize_for_size` (−0.06 KB).

Cost: the toolchain needs the rust-src component (wasm-build-env.sh fails with a message
when it is missing; the docs workflow installs it), and a clean module build compiles core
and alloc (a few seconds). DJS_STABLE_ONLY=1 drops all of it and builds today's module on
plain stable (450.5 KB without D2333 too). Exit plan: if cargo or rustc change the
flags, drop the lines in wasm-build-env.sh and raise the ceiling by the recorded 10.8 KB
under this decision.

## D3091: Intrinsic names and signatures in compact run-time tables
date: 2026-11-05 · by: bytes · issue: 0225
`bytecode::intrinsics::INTRINSICS` (name, kind, arity per intrinsic) is a `const` for the
compile-time users (`const_id`, the hash, the registry tests); the run time reads
`SIGNATURES` (one byte per intrinsic: params, variadic, kind), `NAMES` (all names back to
back) and `NAME_ENDS` (u16), built from it in const. Indexing the `const` at run time had
put the whole 12-byte-per-entry table in the module, with a pointer and length per name.
`bad_argument` takes the intrinsic id and looks the name up in its cold path (one
register per site instead of a pointer and a length); `bad_argument_named` covers the
two non-intrinsic names. The literal runtime lookups by name became `const_id`.
Messages are unchanged. Module 431,762 → 430,963 gzip (−0.8 KB; data −3.5 KB raw),
instructions within ±0.8% on the 12 workloads.

## D3092: Opcode names apart from the instruction table
date: 2026-11-05 · by: bytes · issue: 0225
`OpInfo` has no `name`; the names are `bytecode::OP_NAMES` (a `const` array, by opcode
byte), which `Op::name`, the disassembler, the opcode hash (`hash_ops(OPS, &OP_NAMES)`,
the same value as before) and djs-docs read. The interpreter indexes `OPS` at run time,
which puts the whole table in the module; with the names in it, 214 name pointers and
2.4 KB of names went along although nothing in the shipped module prints them. The
three internal-error messages that named an opcode ("bad bytecode: ...", "not supported
yet: ...") say `op N` in the shipped module (`detail::op_name`, like D2336's details)
and the name elsewhere; debug assertions use `Op::name`. The dump module's global scan
matches opcodes instead of names. Module 430,963 → 429,322 gzip (−1.6 KB),
instructions within ±1.1% on the 12 workloads.

## D3093: The compiler names intrinsics at compile time
date: 2026-11-05 · by: bytes · issue: 0225
The compiler emits calls to about 40 intrinsics for ordinary code (destructuring,
for-of, generators, classes, decorators, `using`, `import()`), and looked each one up by
name at every emitted call: a linear search over the 452 names. `intrinsic_id_of!("Name")`
(bytecode/intrinsics.rs) is `const { const_id("Name") }`, so the id is a constant and an
unknown name fails the build. Run-time lookups by name remain where the name is data:
privileged `%Name` calls and `%InstallIntrinsic`. typescript.js compiles in 6.2% fewer
wasm instructions (4,204 → 3,944 M), codemode bodies 0.2–2.5% fewer; module −0.8 KB gzip
(429,322 → 428,507).
## D3080: Arena::truncate moves the zeroed frontier back
date: 2026-11-05 · by: perfreg · issue: 0224
Freeing the end of the arena (the compactor's last step) truncates the backing Vec instead
of zero-filling the freed bytes; the capacity stays. The arena's invariant (bytes past
`len` up to the Vec's length are zero) holds either way, and the next allocations re-zero
16 KiB ahead as before. Zero-filling cost every collection a memset of its garbage and
left the frontier far ahead, so all later allocations went through the in-place path,
which for converted string data (`alloc_from_iter`) is a byte loop: `JSON.stringify` of
10k objects was 1.5x slower after the in-place collector (ddd792d6f2). Guard:
`arena::tests::truncate_moves_the_frontier_back`.

## D3081: Regexp follow sets only where a STAR uses them; allocation ceilings
date: 2026-11-05 · by: perfreg · issue: 0224
The compiler computes a concatenation's follow sets (D1501, D1563) only from its first
element that can hand one to a `STAR` loop (`wants_follow`), moves the accumulated set
instead of cloning it, and tests disjointness without building sets. The bytecode is
unchanged (5,033 patterns compared). Computing them for every element had doubled the
compile time of short patterns. crates/djs-core/tests/allocs.rs fixes ceilings on the
allocations per compile of djs-bench's patterns (a counting global allocator): a
deterministic guard where compile timings are noise in the gate. Lower counts lower the
ceilings; a higher one needs a reason and benchmark numbers.

## D3082: Slot plans drive tracing; the compactor updates and slides in one walk
date: 2026-11-05 · by: perfreg · issue: 0224
`walk::for_each_slot` follows per-kind `SlotPlan`s (the traced fixed fields and tail
fields, built at compile time from the layout table; checked against the layout walk by
`slot_plans_match_the_layouts`) instead of matching every field per cell. The compactor's
passes 3 and 4 are one walk over the runs of live cells: update each cell's slots, then
move the run, since forwarding needs only the bitmap and the block table. A template's
inline-cache offsets (D1621) are emptied when the marker traces it, because the walk may
have moved the cache cell already. Cells, sizes and order are unchanged (no image
change). Instructions per full collection of 200k records 350M → 238M (Cheney before
0101: 296M), a 500k-node list 724M → 593M (571M).

## D3083: Benchmarks compare per collection, and the shipped module's instantiation
date: 2026-11-05 · by: perfreg · issue: 0224
`gc/<workload> collection` (ms for a fixed workload) is the GC number to compare across
versions; `per MiB live` comes from the same samples and rises whenever cells get smaller
(the 200k records went from 60 to 34 MiB, so ms/MiB rose 60% while collections got
faster). `wasm/instantiate` measures the `compiled-prelude` build, dominated by its 515 KB
data segment; `wasm/instantiate shipped` is the module hosts load. `engine/djs/empty` is a
VM plus the shared prelude since 0044, not a bare VM.

## D3094: Wasm builds run the prelude only with djs-core's `prelude` feature
date: 2026-11-05 · by: bytes · issue: 0225
The `%Install*` intrinsics (InstallConstructor, InstallPrototype,
InstallPrototypeConstructor, InstallAccessor, InstallValue, InstallMethod,
InstallIntrinsic, MarkNative, MarkFast) and what only they use (`Intrinsic::prototype_of`,
the install helpers, `fast::index`, the `@fast` entries' names) run only while the
prelude builds a realm. A wasm module starts every VM from the base image, so it never
calls them, but the intrinsic dispatch kept all of it reachable. djs-core's build script
sets `cfg(djs_prelude)` natively always and in wasm only with the new `prelude` feature
(djs-wasm's `compiled-prelude` turns it on for `vm_new_compiled`). Without it those
intrinsics fail with "%Name is not in this build", and `Vm::new` with
`VmOptions::prelude` panics with a message naming the feature. The feature is code only:
image neutral (compat_inputs.rs) and outside the build hash, so the base made by the
native build script still matches. Module 428,519 → 423,617 gzip (−4.9 KB); pi's tests
on the shipped module 221/226 as before.

## D3095: The run time reads a cell kind's minimum size, not its layout
date: 2026-11-05 · by: bytes · issue: 0225
`CellKind::min_size()` is a constant per kind (generated by `cells!` from each
`LAYOUT.min_size`). Allocation (`Heap::alloc_cell` and its siblings) and the heap walk's
size check read it instead of `kind.layout().min_size`, which indexed the `LAYOUTS`
table at run time and so put every `CellLayout`, `FieldInfo`, `TailInfo` and their names
in the module. Since the GC traces from compile-time slot plans (0224), nothing else at
run time reads the layouts; `layout()` stays for tests, the layout hash and djs-docs.
Module −1.7 KB gzip, instructions within ±1.1% on the 12 workloads (with D3094).

## D3084: Instruction ceilings for work the gate cannot time
date: 2026-11-05 · by: perfreg · issue: 0224
crates/djs-core/tests/instructions.rs measures the machine instructions a `Vm::new`
(building the realm and compiling the prelude from source) takes: the test binary
re-runs itself under `/usr/bin/time -l` with the work and without it, the lower of two
runs each. Instructions retired repeat to 0.1% whatever the load, where times in the
gate (everything in parallel) do not. The ceiling is the count measured when it was set
plus 20%, per profile (dev, which the gate runs, and release), for growth of the prelude
and the builtins; a count above it is a regression unless that growth explains it, and
raising it takes the numbers. D3091's name lookup measured 198M against 136M. Only macOS
reports the counter; elsewhere the test says so and passes. Further ceilings of this kind
go in the same file.

## D3100: The shipped and slim wasm modules read the arena unchecked (amends D2520)
date: 2026-11-06 · by: heapx (owner's decision) · issue: 0226
`scripts/build-wasm.sh` (default, `--keep-names`, `--slim`) builds djs-wasm with
`unsafe-heap`; so do the gate's `build:size`/`build:size-slim` and their release clippy
legs, djs-bench's size and wasm groups, scripts/perf/setup.sh and scripts/size/variant.sh,
so every shipped-module measurement is of the module that ships. Everything else keeps
the checks: native builds (no sandbox: unchecked access would be memory unsafety in the
host's process), the dump and docs modules (the dump module walks the heap the most), and
every test, difftest, fuzz, e2e and gate build except the legs on the shipped modules
(size budget, pi's tests on slim and in next mode). D2520's condition for the feature,
"trusted image storage", now always holds: heap images never leave the process (D2560),
and persistence is JavaScript dumps that are compiled like any script. In wasm the
instance's linear memory confines an engine bug to that instance; the checks only turned
in-instance corruption into a clean trap, so with them off a bug may instead leave that
VM's state wrong, never the host's. The unchecked build stays covered by the gate's
unsafe-heap legs (verifier fuzz and e2e with the debug assertions; the corpus in release
with suspension and GC stress, where nothing checks and a silent corruption shows as an
output difference) and now by pi's tests on both shipped modules. Running
native-vs-wasm on the shipped module was not cheap: its images differ from the native
driver's default-feature build, so it would need a driver built to match.
D2520 saw +2-3% instructions on tight and call-heavy loops; this measured tight +4.3%,
loop +4.4%, calls +3.5%, call1 +2.8%. The cause: `interp/run/hot.rs` read and wrote frame
slots with its own slice checks (`b.get(o..o + 8)`), so the tier-0 loop's hottest
accesses stayed checked while the unchecked inlined heap code changed V8's register
allocation of `run_hot`. Its `ld`/`st`/`ld32`/`window` now go through `access.rs` (the
module's own doc already said the tier-0 loop's slice does), unchecked with the feature
and one u64 compare without. Against main (wmeasure, n=5), the shipped module: tight
−4.5%, loop −4.7%, calls −9.6%, call1 −7.7%, prop1 −9.3%, pi-like −8.1%, json −10.4%,
arrays −17.1%, pushpop −20.4%, indexof −30.0%, all 21 workloads −4.5 to −30%; the
checked build gains too (tight −4.8%, calls −6.3%, json ±0.2%); slim (n=3): tight −4.9%,
loop −4.4%, calls −10.0%, call1 −7.6%, pi-like −8.0%, arrays −17.3%. Size gzip −9: shipped
422,453 → 414,390 (−8.1 KB), slim 357,078 → 348,676 (−8.4 KB); ceilings lowered.
`status.py bounds` now builds the checked module and reports what its checks would cost
against the shipped one.

## D2842: The npm package is @earendil-works/djs, version 0.1.0; dumps carry no version
date: 2026-11-06 · by: owner · issue: 0228
The host package publishes as `@earendil-works/djs` (the earendil-works npm scope), starting at
0.1.0. One package holds the shipped, slim and dump modules as entry points (`.`, `./slim`,
`./dump`). The dump header stays `"djs-dump";` without a version marker: the host knows which djs
it loads a dump with. The pi adapter is not part of the package. Only npm for now (no crates.io).

## D3105: The npm package is built by tsc into target/npm/package and packed from there
date: 2026-11-06 · by: npm · issue: 0228
`scripts/npm-pack.sh` builds `@earendil-works/djs`: the three release modules (or the files
given with `--modules`), then tsc (TypeScript 5.9.3, js/package.json's only devDependency, pinned
by js/package-lock.json and installed with `npm ci` when it differs) emits dist/*.js and *.d.ts
from js/src with `rewriteRelativeImportExtensions` (js/tsconfig.build.json); the declarations'
`./x.ts` specifiers are rewritten to `./x.js` afterwards (tsc keeps them). The generated
dump-runtime.js is copied with a hand-written dump-runtime.d.ts (helpers, script, scriptFor):
tsc's inferred declarations of it needed `esnext` libs in every consumer. The package directory
gets dist/, wasm/{djs,djs-slim,djs-dump}.wasm, js/README.md, both license files and js/package.json
minus `scripts` and `devDependencies`; `npm pack` runs there. js/ itself stays the source (node
runs its TypeScript in the repository; node refuses to strip types under node_modules, so the
package ships JavaScript). tsc rather than node's type stripping: the declarations need tsc anyway,
and it type-checks the package on the way (js/tsconfig.json now checks clean).

## D3106: Each entry point is the whole API with its own module; Sandbox.defaultModule
date: 2026-11-06 · by: npm · issue: 0228
js/src/index.ts (`.`), slim.ts (`./slim`) and dump.ts (`./dump`) re-export sandbox.ts and
override `loadModule`, `defaultWasmPath` and `Sandbox` (a subclass whose static
`defaultModule()` names the entry point's module), each with a literal `new URL("../wasm/NAME.wasm",
import.meta.url)` in its own file, so that bundlers (webpack, Vite: tested) copy exactly the
modules an application imports. `create` and `resume` use `this.defaultModule()`; the synchronous
constructor finds a finished `loadModule` of that location (compiled modules are kept by location,
which replaces the one "loaded default"). One process can use several entry points at once (the
main one with the dump one as `dumpModule`): the defaults are per class, not global. `$DJS_WASM`
overrides only the main entry point (the slim and dump ones are not interchangeable with it).
sandbox.ts itself defaults to `$DJS_WASM`, else, from source (`.ts`), the repository's build, written
so that no bundler takes it for an asset; built, it has no default. Environment reads go through
env.ts, which asks Deno's permissions first (an unpermitted read would prompt or throw). A location
string is a file path in node, Bun and Deno and fetched in a browser; a `file:` URL (or string)
is read, anything else fetched.

## D3107: What the package exports and ships
date: 2026-11-06 · by: npm · issue: 0228
Exports: `.`, `./slim`, `./dump`, `./dump-runtime` (hosts that continue dumps outside djs need
its helpers), `./wasm/*.wasm` (for hosts that place the modules themselves) and `./package.json`.
Not exported: `./pi` (D2842), `./wire` and `./reader` (the Sandbox's internals; their formats move
with the ABI, so exporting them would freeze them). `engines` stays node >= 20: the smoke test
passes on node 20, 22, 24 and 26; Deno needs `--allow-read` (for the package's directory). The
license files are LICENSE-MIT (copyright Earendil Works Co.) and LICENSE-APACHE (the canonical
text), in the repository root and in the package (named in `files`: npm adds only `LICENSE*`
files without a suffix by itself). `sideEffects: false`.

## D3108: The publish workflow tests the one packed tarball everywhere, then publishes on a tag
date: 2026-11-06 · by: npm · issue: 0228
.github/workflows/publish.yml builds the tarball once (scripts/npm-pack.sh on stable Rust with
rust-src: the shipped build environment), runs `npm publish --dry-run` and hands the tarball to a
test matrix: scripts/npm-test.sh installs it in a clean directory and runs js/pkg-test/smoke.js
(every entry point by the package's name: runs, async calls, TypeScript, errors, slim, a dump and
resume in the dump module and through `dumpModule`, the dump continued in the host's engine) on
node 20, 22 and 24, Bun and Deno; js/test's tests (minus the internal modules' and pi's) and the
sleep-dump example on the package on node 22 and 24; and smoke.js in headless Chromium as native
modules (an import map), bundled by Vite and by webpack. A tag `vX.Y.Z` must match
js/package.json's version; it publishes (`npm publish TARBALL --provenance --access public`) in
the `npm` environment, for the owner's approval, by npm trusted publishing (OIDC) or an
`NPM_TOKEN` secret there. A manual run stops at the dry run unless `publish` is ticked.

## D3109: The gate packs the package and runs its smoke test in node
date: 2026-11-06 · by: npm · issue: 0228
The `npm package` leg (class nodocs; its key also has npm's version) runs scripts/npm-pack.sh
with the gate's builds, the size and size-slim modules as they ship and the dev module with the
dumper standing in for the dump module, then the smoke test in node on the tarball: about 4 s
on top of builds the gate makes anyway, and it catches a package that no longer builds (tsc type
errors included), packs, resolves or runs. TypeScript comes from the npm cache after the first
`npm ci` in a worktree (js/node_modules stays: the gate's clean keeps ignored files).

## D2843: The MIT copyright holder is Earendil Inc.; npm releases use trusted publishing
date: 2026-11-06 · by: owner · issue: 0228
LICENSE-MIT names Earendil Inc. (amends D3105's "Earendil Works Co."). The package's homepage is
https://earendil-works.github.io/djs/. Releases publish from `.github/workflows/publish.yml` on a
`v*` tag through npm trusted publishing (GitHub OIDC, provenance), gated by the `npm` GitHub
environment (required reviewer, `v*` tags only); a repository ruleset lets only admins create,
move or delete `v*` tags.

## D3110: The engine comparison runs every engine in wasm in one node process, a fresh VM per run
date: 2026-11-06 · by: benchpage · issue: 0229
js/bench/engines/run.ts compares engines as an embedder runs them: each engine's wasm module is
compiled once per process (timed apart), every run gets a new instance and VM (isolated, its
linear memory measured alone), and the times are medians of 25 runs after 3 warm-up runs, so V8
has tiered the code up (a long-lived host). The tool answers are recorded in node first
(tests/bench/codemode/tools.ts) and given to every engine as one JSON string `__DATA`; the stub
tools in the engine stringify their arguments and parse their answer's JSON text, the work a host
bridge does, identically for all engines (a bridge through each engine's own host API would
measure the APIs, not the engines; js/bench/pi.bench.ts compares the host bridges). Every
engine's result must be the string node computes, or the run fails. Measured: module bytes (raw,
gzip -9, brotli 11; JS glue apart), compile time, idle VM memory, new-VM time, run time and
memory after the run per workload, and for djs the dumps at every idle point through the Sandbox
with async host tools (shipped module plus dump module): write time, raw and gzip size, the time
of a new sandbox with the run loaded. Wall-clock medians, not instruction counts: the
comparison spans engines and hosts whose instructions do not compare, and the load is recorded
(`engines/load`; record below ~4).

## D3111: The QuickJS releases are built with wasi-sdk by one script, as a minimal reactor
date: 2026-11-06 · by: benchpage · issue: 0229
Engines: djs (shipped module), quickjs-wasi 3.6.2 and quickjs-emscripten 0.32.0's two
release-sync variants (Bellard 2025-09-13, QuickJS-NG 0.12.1) from npm (package-lock.json pins
them with integrity hashes), and the latest releases, QuickJS 2026-06-04 (bellard.org tarball)
and QuickJS-NG 0.18.0 (its release amalgamation), built by scripts/bench-engines.sh with wasi-sdk
34 (sha256-pinned per platform), `-O2`, as WASI reactors around js/bench/engines/driver.c (one
runtime and context, a global string, eval, drain the jobs, read `__result`): the same compiler
and embedding for both, nothing of quickjs-libc. Bellard's sources need three edits for WASI (no
Atomics without threads, no malloc_usable_size, an unused setjmp.h), applied by sed; QuickJS-NG
builds as released. Everything lands in vendor/engines of the primary checkout.

## D3112: monty runs as its wasm component, instantiated on the calling thread
date: 2026-11-06 · by: benchpage · issue: 0229
@pydantic/monty 1.3.0's wasm build is a component (jco) that the package runs only in worker
threads. The harness instantiates the same component modules on the calling thread with the same
capabilities (no preopens, environment or network; exit denied) and drives its `dispatch`
(`configure`, one `feed` per run), so its times have no thread hop the JS engines do not pay.
The workloads' Python translations (tests/bench/codemode/*.py, by hand) must return the same
string as the JavaScript, like every other engine.

## D3113: The codemode workload set is synthetic; the sessions it was derived from stay private
date: 2026-11-06 · by: benchpage · issue: 0229
tests/bench/codemode holds nine invented bodies, one per cluster of the codemode bodies found in
the owner's agent sessions (work/research/0229: counts and size distributions only), with stub
tools whose data comes from fixed word lists and a seeded generator. Nothing from the sessions
(bodies, paths, contents, prompts, outputs, tool or server names) is committed; the extraction
writes only to gitignored scratch. The gate's `codemode workloads` leg runs the set on djs's dev
modules once (about 2 s), with a dump and a load at every idle point, so the page's workloads keep
working.

## D3114: The benchmarks page's tables are generated from `engines/*` status keys
date: 2026-11-06 · by: benchpage · issue: 0229
`scripts/status.py engines` runs bench-engines.sh's setup and run.ts three times and records the
medians as `engines/<engine>/...`, `engines/<engine>/<workload>/{run-ms,memory}` and
`engines/djs/<workload>/dump-*` keys plus `engines/load`; djs-docs renders them in `gen:status
engines-size|engines-startup|engines-run|engines-memory|engines-dumps` (the engine and workload
lists are constants in crates/djs-docs/src/status.rs), with the commit, date and load. The
`recorded` region leaves these keys out: they are the page's tables.
## D3140: Below the GC threshold the arena grows by an eighth, in whole pages
date: 2026-11-05 · by: pmem · issue: 0234
`Arena::reserve` grows the capacity to `cap + max(cap/8, 64 KiB)`, rounded up to 64 KiB, not
to `2 * cap` (still capped by the GC target, D1870). In wasm the arena grows in place at
memory's end (D2053), so doubling bought nothing but pages a run might never touch, and the
benchmark runs that end below their first threshold paid for them: sequential 896 → 704 KiB,
text 1344 → 1216, json 1856 → 1472. Natively the more frequent reallocs cost nothing measurable
(scripts/perf/w workloads within ±0.2% instructions).

## D3141: A VM's arena starts at the image in whole pages; the page allocator's HUGE is 128 KiB
date: 2026-11-05 · by: pmem · issue: 0234
`Heap::image_capacity` is the image rounded up to 64 KiB pages, no longer the image plus a page
(D1942, D2541): idle memory 640 → 576 KiB. For the arena to grow in place from there, the base
buffer must be a page run (a huge block) and the top block: `HUGE` drops from 256 to 128 KiB (the
base is ~182 KB, the slim one ~132 KB; `page_block` still lifts a smaller one to HUGE), so a
192 KiB base buffer no longer starts as a medium block that moves when it outgrows HUGE. Blocks
of 128–256 KiB never fit dlmalloc's 128 KiB first segment anyway.

## D3142: Big staging requests get their own block, placed after room for the arena
date: 2026-11-05 · by: pmem · issue: 0234
`djs_staging(len)` for `len` ≥ 16 KiB frees the old buffer and allocates a fresh one with the
page allocator told (`with_top_room`) to start page runs placed right behind the arena `2 * len`
later; a smaller request drops a big buffer. Before, the staging buffer stayed at its high-water
size right behind the arena, and decoding a big wire value (a tool result, a non-Latin-1
string: UTF-8 can double as UTF-16) moved the arena past it, leaving its old pages as a hole:
fanout 2496 → 1792 KiB. The gap pages are free; the arena grows into them.

## D3143: The mark stack reserves 16 KiB up front
date: 2026-11-05 · by: pmem · issue: 0234
`STACK_RESERVE` (4096 entries) is what a collection reserves before marking; the stack still
grows on demand to `max(len/64, STACK_CAP)` entries. The 64 KiB reserve did not fit dlmalloc's
first segment next to the other side tables, so it became a page run at memory's end, one more
page for every run that collected.

## D3144: Large live leaf cells count for 40% of the next collection's growth
date: 2026-11-05 · by: pmem · issue: 0234
`next_threshold_cheap`: the mark sums the live leaf cells (Str8, Str16, Bytes, U32Array, BigInt)
of at least `CHEAP_LEAF` = 16 KiB above the old region, and the growth after the collection
leaves out `CHEAP_DISCOUNT` = 60% of them. They cost a collection a mark bit and their bitmap
words, so growth proportional to them bought headroom no collection needed; codemode runs hold
tool output as big strings. heavy 4864 → 4480 KiB; native strbuild +1.6% instructions,
json-indent ~+1.5%, everything else ±0. Chosen within a 2% CPU budget (coordinator): 75% of
leaves ≥ 1 KiB gave heavy 3584 KiB but json-objects +5.6% (JSON.stringify's transient 8–16 KiB
output pieces were discounted: 18 collections instead of 13); 75% at 16 KiB, strbuild +2.2%.
The discount cannot tell long-lived from transient big strings; that would need a survivor mark
in the image (follow-up). Minor collections applying the slice-detach rule (D2542) were
measured and dropped: no gain on the benchmark workloads, pi-like +0.9%.

## D3120: Pure leaf intrinsics run in the tier-0 loop
date: 2026-11-06 · by: mathfast · issue: 0230
A `call_intrinsic` that the hot loop did not know (everything but a few type tests) left
run_hot for `step_once` → `call_intrinsic`: about 550 wasm instructions per call, more than
most leaves do (Math.floor made two such calls, substring six). `Exec::pure_leaf`
(interp/run/intrinsics.rs) now holds the leaves that compute a value from at most three
arguments with no effect on frames, stack or fuel and none before a fault: type tests and
`As*`, the conversions of primitives, `StrLength`, the `Str*` string leaves, `Number*`
formatting and parsing, `SameValue`, `PrimitiveValue` and the libm leaves. The hot loop
calls it for the ids in `hot::PURE_LEAVES` (a bitset; a test checks it against
`pure_leaf`), so other intrinsics decline without a call. A fault declines and
`call_intrinsic` runs the leaf again and throws (leaves change nothing before they fail).
A host refusal stops the run at run_hot's exit (D1690). `call_intrinsic` tries `pure_leaf`
last, so the intrinsics it does not cover pay nothing. Running every declined intrinsic
through `step_once` in place (no loop exit) was measured first and gained nothing: the
cost is in the general dispatch, not in the exit.

## D3121: djs-bgen inlines small helpers into the bytecode prelude
date: 2026-11-06 · by: mathfast · issue: 0230
TS builtins are built from many tiny helpers, and every call costs a frame plus the call
and argument bytecodes (substring: 8 helper frames). The JS emitter (emit_js.rs) now
replaces a call of an internal helper (no installs, so it never shows in stack traces; no
`this`, rest, `new.target` or `%ArgumentCount`; not `@native`, `@fast` or `@frame`;
≤ 8 statements; 4 levels deep at most) with one expression when its body has one:
`const`/`let` and assignments become `(t = e, …)` with caller temporaries `$1..`, `if` with
`return` becomes `?:`, `throw e` becomes the new leaf `%Throw(e)`. Arguments go into
temporaries in order (names and literals are used directly), so evaluation order and
count are unchanged. A call is inlined only if its expansion is ≤ 160 characters of JS:
every call site is a copy in the base image, and 160 kept most of the gain for ~1 KB gz
(260 gave little more for 1.4 KB more). The Rust backend already inlined (emit_rust.rs);
this is the bytecode backend's counterpart. `@frame` (new tag) keeps a helper's frame where
the engine looks for it: `CallPrepareStackTrace`, whose frame stack.rs searches for. Helpers
that every caller inlined disappear from the prelude after the base snapshot's collection;
the registry tombstones them (73 `%prelude:` entries).

## D3122: Math.max / Math.min get @fast entries again
date: 2026-11-06 · by: mathfast · issue: 0230
0159 (D2310) dropped them for want of calls in the workload set. The 0229 set has
`lines.reduce((m, l) => Math.max(m, l.length), 0)` over ~30k lines (heavy) and a
`Math.max` per row (json), at 8.8k wasm instructions per call (a rest array, CoerceNumbers,
a Get loop) against ~1.4k in quickjs-wasi. The entry (fast.rs `extremum`) takes primitive
arguments whose ToNumber is pure. It converts all of them before it answers, so an object
after a NaN still declines to the builtin. Cost ~150 B gz; Math.max(i, 3) 8,838 → 1,206 wasm
instructions. Handling `@fast` calls inside run_hot's call arm instead of at its exit was
measured and dropped: +4–9% on @fast calls, but −7–10% on every plain closure call
(register pressure in the call arm, D1352).

## D3123: The size ceilings rise for 0230
date: 2026-11-06 · by: mathfast · issue: 0230
Approved by the coordinator. Shipped module 414,524 → 416,327 B gz (+1,803), slim 348,793 →
350,181 (+1,388), measured on the branch against main at 42ad8afd. The ceilings rise by those
deltas (417,059 and 350,904). Most of it is the inlined helpers in the base image, the rest
the pure-leaf path and the Math entries. Wasm instructions per operation (scripts/perf/
wmicro.py, main → 0230): Math.max(i, 3) 8,838 → 1,206; Math.floor 3,531 → 2,118; substring
13,326 → 8,206; at 9,386 → 5,895; padStart 17,424 → 12,503; repeat 9,839 → 6,375; toFixed
8,409 → 5,492; trim 4,396 → 3,340; toLowerCase 5,206 → 4,022; startsWith 12,933 → 8,720;
lastIndexOf 9,571 → 6,029; replace 18,589 → 14,926; parseInt 4,899 → 3,835; Array.isArray
1,819 → 1,347; String(i) 6,440 → 5,979. Plain calls, push/pop, slice, charAt and Object.keys
are unchanged (±2%).
## D3135: The RegExp search is the matcher's; programs are read in place; one exec state serves many matches
date: 2026-11-06 · by: pre · issue: 0233
The search around a pattern is no longer bytecode (`SPLIT_GOTO; SKIP; ANY; GOTO` and `SCAN`,
both opcodes gone): the matcher's `PC_FIND` state looks for the next start and runs each
attempt on an empty backtrack stack, so slot writes outside any frame are not logged (the
captures are cleared before the next attempt). The header (`H_LEN` 7) has the first set (ranges
plus a bitmap of its Latin-1 characters, `H_SEARCH`), the mode (`M_ANCHOR`: a non-multiline `^`
on every alternative means one attempt where the exec starts, a multiline one means attempts
only at line starts, found by skipping to the next line terminator; `M_SKIP`, `M_NAMES`) and the
leading STAR's skip register. A STAR followed by the end of the pattern (`F_LAST`) pushes no
frame. Fuel stays additive: passing a position costs one unit and is paid before moving on.
`exec_in`/`resume_in` take a `Prog` (the RegExp's U32Array cell read as byte quads, no copy)
and an `ExecState` the caller keeps (slots, generations and stack in one Vec): replace runs all
its matches in one state and one borrow of the program and string, and builds its output one
byte per unit when the string and the template are one-byte, sized to the input. The program
encoding and the saved state changed: COMPAT_VERSION 5. Per exec 5 allocations became 1, per
match in a replace 0.

## D3136: RegExpCreate shares a RegExp's program; pristine RegExps skip the flag getters and matchAll's Construct
date: 2026-11-06 · by: pre · issue: 0233
`%RegExpCreate(proto, pattern, flags)` accepts a RegExp object as the pattern: with equal flags
the new RegExp shares its program cell (programs are immutable), else it compiles its source.
RegExpConstructor uses it for a RegExp pattern (after every observable step), and its first
lines take `RegExp(string[, string])` with new.target undefined or %RegExp% straight to
RegExpCreate (IsRegExp of a string, %RegExp%'s fixed `prototype` and ToString of strings
observe nothing). `get flags` returns [[OriginalFlags]] when IsPristineRegExp holds (the eight
getters are then the originals), and RegExp.prototype[@@matchAll] on a pristine RegExp skips
SpeciesConstructor, the flags Get and Construct. EscapeRegExpPattern returns the source itself
when nothing needs escaping.

## D3137: %RegExpExecOwn: RegExpExec without its lookups when they read only data
date: 2026-11-06 · by: pre · issue: 0233
`%RegExpExecOwn(r, s, test, state)` runs RegExpBuiltinExec when r is a RegExp without an own
`exec`, its prototype is %RegExp.prototype% whose `exec` is the original data property, and its
own lastIndex is a Number; otherwise it returns undefined and has done nothing, and the caller
takes the spec steps. exec, test and RegExpExec (match, matchAll's next, the slow replace and
split) call it first. Suspendable like RegExpExecRaw; a resumed call skips the checks (they held,
and nothing ran since). New intrinsic: INTRINSICS_HASH changes.

## D3138: A RegExp compile cache of 8 entries in a realm slot
date: 2026-11-06 · by: pre · issue: 0233 · approved: coordinator
`Intrinsic::RegExpCache` (realm.rs, the last entry; RECORDS_HASH changes through the enum) is
undefined or an internal ValueArray of 8 `[source, flags, program cell]` triples, most recently
used first, keyed on exact string equality of the flat source and flags (length first). A
RegExpCreate that would compile looks there first; a miss compiles and inserts at the front,
except sources over 256 code units and programs over 1024 words. Only RegExpCreate reads it: no
property or user code reaches it, the registry and the dump tables do not name it (like
KeptObjects), and the plain-dump fuzzer's regress passes with and without --lazy (its one
--lazy finding, seed 342347, fails the same on main). It keeps at most about 36 KiB alive,
typically 3-4 KiB (programs of the bench patterns are 130-600 bytes). Being in the arena, it is
part of images, and runs stay deterministic (difftest --determinism passes).

## D3139: Size of 0233; the size ceiling rises by what it measured
date: 2026-11-06 · by: pre · issue: 0233 · approved: coordinator
Measured at the merge (main 373e9422 vs the branch, scripts/size/budget.py): shipped module
416,548 -> 418,758 gzip (+2,210; 955,480 -> 959,766 raw, +4,286), slim 350,367 -> 352,495
(+2,128). ceiling_gzip 417,059 -> 419,269 and slim_ceiling_gzip 350,904 -> 353,032: the
ratchet rises by exactly that (1.461x quickjs-wasi, the budget 1.5x has ~11 KB left). Code
bytes by function (perf build with names, merge-base vs branch, +4,088 in all): the matcher
(`resume_in` with the search loops +785 over `resume`, `exec_in` +350 over `exec_at`), the
RegExpExecOwn leaf +752, `create` with the compile cache +1,752 (it was inlined before; the
intrinsic dispatcher -500), `create_like` +336, `groups_object` +654 (was inlined), replace's
`Units` +466 and `units` +374, `first_into` +1,003 for `first_chars` -1,264, the parser's
literal fast path +389, the rest under ±300 each. Trimmed before measuring: one search loop for
one-byte input and one for the rest, generic two-byte conversions (both off the hot paths).

## D3124: @fast entries for the string builtins still over 2x QuickJS after D3120/D3121
date: 2026-11-06 · by: mathfast · issue: 0230
After the general fixes, substring, at, repeat, padStart/padEnd, String(i),
Number.isInteger and toFixed still cost 2-6x quickjs-wasi in wasm instructions. Each now has
a `@fast` entry in fast.rs: primitive arguments only, and every case that would throw
declines, as the D1351 rule asks. `pad` also declines for pads over 64 Ki units and for
two-byte fillers; those go through the builtin's StrRepeat. It writes the run of fillers
into one buffer, then joins it with the string. String() runs through the entry only when
called: a construct never takes a fast entry. Its builtin reads `%ArgumentCount` instead
of a rest array, as reduce does. Wasm instructions (wmicro, before → after, quickjs-wasi in
brackets): substring 8,186 → 2,085 (1,925); at 5,821 → 2,270 (1,608); repeat 6,428 → 2,884
(2,103); padStart(8, '0') 12,505 → 3,719 (2,392); padEnd(20) 14,435 → 3,686 (2,458);
String(i) 6,001 → 1,898 (1,598); Number.isInteger 3,683 → 1,006 (1,236); toFixed 5,496 →
3,391 (2,566). Plain calls are unchanged within the ±5% noise of wmicro. Size: +1,302 B gz
shipped, +1,309 slim (420,061 and 353,804 against main's 418,759 and 352,495); the ceilings
rise by those amounts. Opt-in inlining (an `@inline` tag) of ClampPosition, RelativeIndex
and ToLength was measured and dropped. Before the entries it bought substring 11%. After
them it bought nothing measurable (a.at, a.lastIndexOf, substr ±1-5%) for 330 B gz.
Inlining ToString, which loses its fast entry, made trim, repeat and toLowerCase 3% slower:
two pure leaves cost more than the `@fast` exit.
## D3130: await and async return subscribe and settle in the op where no user code runs
date: 2026-11-06 · by: pasync · issue: 0232
`await v` subscribes the async state in the op itself (perform_then on `v`, or a reaction job
with `v` for a value that fulfills a new promise) when Await's steps cannot run user code: `v` is
a Promise whose `constructor`, found as a data property without a getter or Proxy on the way, is
%Promise% (PromiseResolve returns it), or `v` is no object or has no callable data `then`
(`promise::plain_promise`, `plain_fulfillment`). The intermediate promise of the spec is left out:
nothing else can reach it. `async_return` of such a value and `async_reject` settle the result
promise in the op (%AsyncFunctionThrow is gone); anything else still tail-calls %AsyncAwait /
%AsyncFunctionReturn, so the order of jobs is the spec's. `next_job` runs a PromiseReactionJob
whose handler is an async function state by resuming the state as the job's bottom frame
(`generator_resume` with `Continuation::Job`) instead of entering the builtin. The heap after
each fast path is what the builtins would leave, so dumps (taken at idle points, where no frame
is live) are unchanged. Measured (scratch amicro.py, node --no-liftoff, machine instructions per
op): await 1 47 k → 23 k (qjs-2026 9 k), async f() + await 42 k → 26 k; ops.ts ratios djs/best
QuickJS: await value 2.3 → 0.19, async call + await 1.84 → 0.83.

## D3131: Promise combinators do plain elements in Rust
date: 2026-11-06 · by: pasync · issue: 0232
`%PromiseCombinatorRun(fixed, kind, index, it, next)` takes the elements an Array values
iterator gives without user code (`iter_plan`) and does each element's loop body of
PerformPromiseCombinator (values slot, element Record, resolve/reject element functions,
Promise.resolve and `then` with its derived promise) in Rust while none of it could run user
code: C is %Promise% with the original `resolve` (both kept in the `fixed` Record), the element
is a `plain_promise` or a `plain_fulfillment` value, and `then`, `constructor` and
%Promise%[@@species] are the originals (or the builtins are frozen and the element has no own
`then`/`constructor`). It stops at the first other element, which the TS loop does the slow way,
then calls it again. `next_job` settles a Promise.all resolve element's reaction job (SettleElement
plus resolving the derived promise) in `promise::all_element_job` when both capabilities are
%Promise% promises and the last element's values array has no callable `then`. The heap is
exactly what the builtins leave (the dumper reads combinators from it). allSettled/any elements
and their jobs, and the keyed combinators, keep the TS path for their jobs. Measured: Promise.all
over 20 resolved promises 1 340 k → 331 k instructions (qjs 310 k); ops.ts Promise.all 3.8 →
about 1.1-1.3 under load 105.

## D3132: %RealmIntrinsic reads its name in place
date: 2026-11-06 · by: pasync · issue: 0232
The argument is a Latin-1 literal of the builtins; it is read through `str_view` instead of
`str_to_string`, which allocated a Rust String per call (5-7% of an await before D3130, and
every builtin that makes an object with `RealmIntrinsic("…Prototype")`).

## D3133: Size of 0232; the size ceiling rises by what it measured
date: 2026-11-06 · by: pasync · issue: 0232 · approved: coordinator
Measured at the merge (main 09b8cb8f, after 0230b, vs the branch, same build environment):
shipped module 420,075 -> 422,256 gzip (+2,181; 964,175 -> 969,696 raw, +5,521), slim 353,818 ->
356,145 (+2,327). On top of 0230b's ceilings (D3124): ceiling_gzip 420,571 -> 422,752 and
slim_ceiling_gzip 354,341 -> 356,668: the ratchet rises by exactly that (~1.473x quickjs-wasi,
under the 1.5x budget). Code bytes (perf build with
names): the combinator step with its run loop (D3131) ~2.1 KB, next_job's fast paths (await
resumption, Promise.all element jobs) ~1 KB, the await/return op paths ~0.6 KB, the plain-lookup
helpers ~0.6 KB. It buys await 26.2 k -> 3.8 k instructions (quickjs 9.3 k), async call + await
20.5 k -> 7.2 k (10.1 k), Promise.all of 20 resolved promises 1,323 k -> 220 k (257 k). The
Promise.all element-job path (~0.4 KB gzip) stays: fan-out over tools is a core codemode shape.
## D3125: The comparator sort merges in leaves; only the comparisons run in the state machine
## D3125: The comparator sort merges in leaves; the default sort compares strings in place
date: 2026-11-05 · by: parr · issue: 0231
SortList (builtins/array.ts, `@native`) re-entered its generated state machine for a dozen
resume points per comparison (two Gets, the CreateDataPropertyOrThrow of the merge, slot
traffic): 3.2-3.4x QuickJS's wasm instructions. Now `%SortStart(list)` puts the list's
defined values in an internal null-prototype Array (undefined ones go to the list's end, never
compared), and `%SortStep(st, v)` places the pending comparison's element by `v` and merges
on, in Rust, to the next comparison (true) or writes the sorted values back (false);
`%SortOperand(st, 0|1)` reads its two values. The state (run width, pair start, the two read
positions, the flip and pending flags, the list, the operands, the two regions) is that
array's dense store, so a suspension inside the comparator dumps it like any value. The merge
is the same bottom-up merge as before, so the comparator sees the same calls in the same order
and the sort stays stable; djs-bgen gives SortStep and SortOperand inline forms
(`ex.n_sort_step`, `ex.n_sort_operand`) so they skip the intrinsic dispatch. The default sort
of non-primitives uses the same leaves with ToString/CompareStrings per comparison. Heap
addition: `Heap::dense_store(o)` (the elements cell), for leaves that own their array.
The default sort of primitives compares strings in place: `%SortPrimitives` copied every
element's code units into a `Vec<u16>` through the chunk iterator; it now makes each string contiguous (flattening ropes, which allocates, before any
view is taken), takes the views, and sorts by `string::view_cmp` (bytes for two one-byte
strings, units otherwise); `string::compare` (CompareStrings) uses views too when both are in
one piece. 30k lines: the sort itself at parity with QuickJS (was 1.6x).

## D3127: Map and Set get/has/set/add are @fast entries
date: 2026-11-05 · by: parr · issue: 0231
Map/Set with string keys were 2-2.5x QuickJS, and hashing was not the cause: the keyed hash
is cached in the string and is ~5% of the time. The cost was the bytecode builtin's frame
(ThisMap and the collection intrinsic behind it). `MapPrototypeGet/Has/Set` and
`SetPrototypeHas/Add` now have fast.rs entries: `this` of the right class, else they
decline (the builtin throws its TypeError); CollectionSet allocates (flattening a key,
growing or reindexing the table) before it changes anything observable, so the heap-cap
rule of D1910 holds. +272 B gzip; Map/Set loops with string keys 1.1-1.5x QuickJS (from
2-2.5x), the rest being 0230's call overhead and the string concatenations of the keys.
Size (coordinator-approved trade): 0231 as a whole, measured on main at 0211f403 (after 0230,
0230b, 0232, 0233), adds 925 B gzip to the shipped module (422,242 -> 423,167); the ceilings
rise to the measured sizes (`ceiling_gzip` 423,167, `slim_ceiling_gzip` 356,911) for
D3125-D3129's speedups.

## D3128: `+` of a string and a primitive, and `arr.push(x)`, finish in the hot loop's decline path
date: 2026-11-05 · by: parr · issue: 0231
run_hot's Add handled numbers only: a string operand declined to `step` and `conv::add`; and
every `@fast` call (push most of all) declined, went through `fast_leaf_call` and the generic
`try_fast_call` (an argument array, the entry by function pointer) and `array_push`, whose
`set_element_fast` checked the object four ways before appending. Changing run_hot's own code
moved V8's code for the whole loop: concatenating in the Add arm, or even a `continue` after a
fast call in the exit path, made plain loops and closure calls 4-6% slower (0230 saw the same
for @fast calls in the call arm, D3122). So the loop's code stays as it was but for one call:
the exit path's `fast_leaf_call` became `Exec::decline_fast(op)` (out of line), which runs a
declined `@fast` call as before and a declined `add` of a string and a primitive other than a
symbol (ToString runs no user code and cannot throw; an object, a symbol or the heap cap still
goes to `step`, which keeps D1700's retry). `fast_leaf_call` (now inlined into it) sends a
one-argument call of Array.prototype.push's entry (`fast::ARRAY_PUSH`, a test checks the
index) straight to `Heap::array_append_fast`, which appends when the dense store ends at
`length` (flags, prototypes without index properties, room in the store) with the same fuel
and result as the entry. `Heap::concat` of two flat (or sliced) pieces under ROPE_MIN_LEN
copies them directly instead of walking leaves with two zeroed 32-entry arrays. Measured
against main (0211f403), alternating builds: plain closure calls and loops ±0; push 1.60x ->
1.18x QuickJS; `a + b` of strings 1.73x -> 1.33x.

## D3129: Array.from from %Array% skips Construct; join reads slices in place
date: 2026-11-05 · by: parr · issue: 0231
Array.from called Construct(%Array%, len) (a second builtin call) after ArrayCreate and
`Set(a, "length", len)` at the end. With `this` the realm's %Array% (new leaf
`%IsArrayConstructor`), Construct would make exactly ArrayCreate's array (%Array%.prototype is
non-writable and non-configurable) and the final length Set changes nothing, so both are
skipped. Its array-like reads (`plain_get_index`, D2306) skip objects without
OBJ_INDEXED_PROPS instead of looking the index up in their tables. `join_flat_latin1` accepts
one-byte slices (the lines of a split) besides flat cells, finds each piece's data once, and
writes into the result through one split borrow with a one-byte separator stored directly.
Array.from({length: 20}, fn) 1.2x QuickJS (was 1.8x); joining split lines 1.5x (was 2.0x,
the rest is the collector moving the large results, 0234's area).

## D3126: Amends D3022: any undefined symbol from our crates' objects is incremental corruption
date: 2026-11-05 · by: parr · issue: 0231
D2896/D3022's retry recognized only a missing `.llvm.<n>` symbol in ld64's or GNU ld's
words. 0231's merge failed in build:wasm on rust-lld's `undefined symbol: anon.*.llvm.N`,
which it did not match, and pmem saw native drop glue (`__RINvNtCs.._4core3ptr9drop_glue..`,
no suffix) go missing from `sort_primitives`' object. In a pure-Rust workspace a real
undefined symbol at link time essentially never happens, so `link_corruption` in
scripts/gate.py now treats any `Undefined symbols` / `undefined reference` / `undefined
symbol:` as corruption when the symbol is LLVM-local or the referencing object is one of our
crates' codegen units (`djs_<crate>-<hash>.<cgu>.rcgu.o`: on the line, the line before, or
under ld64's "referenced from:"), and retries once after D3022's clean. scripts/gate-test.py
(leg `gate self-test`) checks it on excerpts of both failures, a GNU ld one, and two that must
not match (a C library's undefined reference, a compile error).

## D3155: Builtin closures keep `length` and `name` lazy
date: 2026-11-06 · by: pasync · issue: 0232b
A builtin closure (`%NewBuiltinClosure`: resolving functions, combinator element functions,
`finally` thunks ...) has three slots now: the builtin, its data and its `length`. Its `length`
and `name` (always empty) are D2445's lazy properties (`OBJ_LAZY_LENGTH`/`_NAME`): `lazy_value`
reads slot 2 and the empty atom for this class, and every other path (own keys first in order,
attributes, redefine/delete materializing them, frozen reads) is the closures' one. Before, each
closure ran two validated `define_own_property` calls and allocated a values block; `new Promise`
made two such closures. COMPAT_VERSION 6 (what a builtin closure's slots and flags mean, and
D3157's async state local).

## D3156: @fast entries for the resolving functions, then, Promise.resolve and GetPrototypeFromConstructor
date: 2026-11-06 · by: pasync · issue: 0232b
`PromiseResolveFunction`/`ResolveWithData` and `PromiseRejectFunction`/`RejectWithData` settle a
Promise data (its own resolving functions, an internal capability) in Rust when the resolution
is a `plain_fulfillment` value (D3130) and not the promise itself, or a rejection; a Record data
(another constructor's capability) declines. `Promise.prototype.then` on a Promise whose
SpeciesConstructor is %Promise% without user code (`plain_species`: a data `constructor` that is
%Promise% and the original @@species getter, or frozen builtins) makes the derived promise and
subscribes. `Promise.resolve` on %Promise% returns a `plain_promise` or a new fulfilled promise.
`GetPrototypeFromConstructor` returns `newTarget.prototype` when it is an object found as a data
property without a Proxy or getter (every builtin constructor benefits). Each mirrors its TS
step by step and declines everywhere else.

## D3157: An async function's state object is made at its first await
date: 2026-11-06 · by: pasync · issue: 0232b
`async_start` allocates just the result promise and keeps it in the state local; the first
`await` (or an `async_return` that needs %AsyncFunctionReturn) makes the Generator object with
that promise and stores it there (`Exec::async_state`). `async_return`/`async_reject` settle the
promise in the local directly when the function never awaited. A suspended frame always holds
the state object, so dumps (taken at idle points, where every activation has suspended) see
what they saw before.

## D3158: allSettled and any element jobs settle in Rust
date: 2026-11-06 · by: pasync · issue: 0232b
`promise::all_element_job` (D3131) takes the element functions of allSettled (fulfilled and
rejected: the `{status, value | reason}` record made with `new_object_from_entries`, whose keys
`status`, `reason` and the values `fulfilled`, `rejected` are well-known atoms now) and `any`'s
reject element, except `any`'s last one, which makes the AggregateError in the builtin.

## D3159: Size of 0232b; the size ceiling rises by what it measured
date: 2026-11-06 · by: pasync · issue: 0232b · approved: coordinator
Measured before the merge (main a805a2a2 vs the branch, same build environment): shipped module
423,181 -> 424,091 gzip (+910; 971,356 -> 973,945 raw, +2,589), slim 356,924 -> 357,805 (+881).
ceiling_gzip 423,167 -> 424,077 and slim_ceiling_gzip 356,911 -> 357,792: the ratchet rises by
exactly that. It buys (wasm instructions per op, quickjs 2026-06-04 in brackets): `p.then(f)`
30.3 k -> ~12-15 k (10.8 k), `new Promise(r => r(1))` 25.6 k -> 14.2 k (5.3 k),
`Promise.resolve(1)` 13.6 k -> 3.8 k (6.9 k), an async call without await 4.3 k -> 3.8 k
(7.4 k), allSettled of 20 662 k -> 233 k (333 k), any of 20 rejected 544 k -> 285 k (279 k).
The bytes: seven @fast entries (D3156), the allSettled/any element jobs (D3158), the async
state at the first await (D3157), four well-known atoms.

## D3160: The construct path's instructions finish outside `step`
date: 2026-11-06 · by: pnew · issue: 0236
Every instruction of `new` left run_hot for `step`, ~1000 wasm instructions each before its
own work (a function returning `new.target` cost 2.1 k vs 1.0 k for one returning 1); `new C()`
took four such trips, a derived constructor eight or more. `Exec::decline_construct`
(interp/run/construct.rs), called from `decline_fast` for its opcodes only (`construct::takes`,
so other declines pay no call), completes: `construct`, `super_call` and `super_call_spread` of
a bytecode function or builtin constructor whose frame fits the stack, the return into a
ConstructResult frame, `special_object` this/new.target/active function, `check_ctor`,
`check_this`, `check_this_uninit`, `check_ctor_return`, `get_super` and `rest`. Each does what
`step` does (fuel, pc, stack including the slots above sp, safepoints, the retry marker) or
changes nothing and `step` runs it (an error, a `prototype` getter, a stack that must grow, the
heap cap); hot_loop_is_invisible now runs classes, derived classes and the builtin
constructors. A cheap decline still costs ~300. Tried and dropped: these arms in run_hot's
body (arrays +1.6%, tight +0.8% instructions), and running on through following trivial ops in
`decline_construct` (slower: they are faster in run_hot). The compiler returns the checked
`this` directly when a derived constructor's body falls off its end; the epilogue
(`check_ctor_return`) is emitted only when a `return` jumps to it.

## D3161: A construct's frame holds its new.target from the entry safepoint on
date: 2026-11-06 · by: pnew · issue: 0236
`construct_call` pushes a bytecode frame with FLAG_CONSTRUCT and new.target in its header
(`push_frame_head` takes it) before the entry safepoint, instead of patching them after `call`
and holding an explicit new.target in a handle across it; the handle remains for native frames.
`construct_with` is split into `construct_this` (the new `this`, or undefined for builtins and
derived constructors, `None` for a `prototype` getter or proxy) and `insert_this`, which
`decline_construct` shares.

## D3162: %ConstructorPrototype; constructor allocations are pure leaves
date: 2026-11-06 · by: pnew · issue: 0236
GetPrototypeFromConstructor (a frame plus %Get with an interned key: 2.7 k of `new Map()`'s
10 k) is an inlined helper around the new pure leaf `%ConstructorPrototype(newTarget,
fallback)`: newTarget's data `prototype` if an object, else the realm's `fallback` prototype,
undefined when a getter or proxy is on the way, then `GetPrototypeFromConstructorSlow` (the old
steps, `@intrinsic`). It replaces D3156's `@fast` entry, which still cost a decline. The hot
loop runs it in place; so are CollectionCreate, PromiseCreate, ErrorCreate, DateCreate,
NewBuiltinClosure (allocations only) and ErrorStack/IsPendingStack (RecordStack's reads), in
`constructor_leaf`, out of line of `pure_leaf` so its string leaves stay inlined.
INTRINSICS_HASH changes.

## D3163: `new` entries for Array, Map, Set and Date
date: 2026-11-06 · by: pnew · issue: 0236
`FastFn::new` is a constructor's entry for `new F(args)` with new.target F (whose `prototype` is
non-writable and non-configurable, so the prototype is the realm's without a lookup): `new
Map()`/`new Set()` with no or a nullish argument, `Array(...)`/`new Array(...)` (a valid length
or the elements; an invalid length declines), `new Date()` and `new Date(v)` for a Number, a
string or a Date. `construct_with` and `decline_construct` try it before `this` is inserted,
both, so the stack slots left above sp are the same either way (else the images differ).
Calls of Map/Set/Date run the builtin (`no_call`). Measured (wasm instructions, quickjs-wasi
in brackets): new Map() 7.4 k -> 2.1 k (2.9 k), new Array(4) 12.6 k -> 1.7 k (3.4 k), new
Date(0) 14.8 k -> 1.7 k (2.9 k).

## D3164: Size of 0236; the size ceiling rises by what it measured
date: 2026-11-06 · by: pnew · issue: 0236 · approved: coordinator
Measured (scripts/build-wasm.sh, main cc33e7d9 vs the branch): shipped 424,091 -> 426,806 gzip
(+2,715; raw 973,945 -> 980,158), slim 357,805 -> 360,485 (+2,680). ceiling_gzip 424,077 ->
426,806, slim_ceiling_gzip 357,792 -> 360,485 (1.489x quickjs-wasi). Code bytes:
decline_construct +2.3 KB raw, the construct_call/construct_this/insert_this split +1.2 KB, the
`new` entries +1.5 KB (Date 825 B), constructor leaves +0.2 KB, ConstructorPrototype +0.3 KB.
It buys (wasm instructions, main -> branch, quickjs-wasi): new C() 4.3 k -> 3.2 k (3.2 k),
new C(i) 5.0 k -> 3.9 k (4.2 k), new F(i) 4.0 k -> 3.1 k (3.8 k), derived default constructor
11.2 k -> 8.9 k (5.5 k), derived `super(a)` 10.7 k -> 7.6 k (6.2 k), Map/Set 7.4 k -> 2.1 k
(2.9 k), new Promise 10.2 k -> 8.5 k (8.7 k), Array(4) 12.6 k -> 1.7 k (3.4 k), Date(0)
14.8 k -> 1.7 k (2.9 k). Plain workloads within +0.6% instructions. Only ~3.2 KB are left to
the 1.5x budget: later speed work has to fund itself.

## D3165: The bridge benchmark: each engine's best path, one operation timed
date: 2026-11-06 · by: bridge · issue: 0238
js/bench/engines/bridge.ts times passing a value between host and VM as the operation an
embedder calls (`BridgeVm.put`/`get` in engines.ts), not as a script's run minus a baseline:
the difference of two runs drowns small payloads in noise. Small payloads repeat within a
sample; every sample is a fresh VM. Each engine gets the best path its API offers, and where it
offers several (quickjs-wasi: JSON text or its handle walk; quickjs-emscripten: JSON text or
building through handles; monty: JSON text with json.loads/dumps or its value arena) each is
measured and each number is the faster path's, since a host that knows what it passes picks it.
The two QuickJS releases get the best C API path in driver.c: JS_ParseJSON straight from linear
memory (no intermediate JS string), JS_NewStringLen, JS_NewArrayBufferCopy, and out
JS_JSONStringify + JS_ToCStringLen read in place. djs goes through the Sandbox's own conversions
(the wire format in, with long ASCII strings copied into a reserved string; the reader out).
Every value must read back equal, and end-to-end cases (a sync host function `tool`) must agree,
or the run fails. The gate's codemode-workloads leg runs it on djs with --quick.

## D3166: Charts are generated with their table, from status.tsv
date: 2026-11-06 · by: bridge · issue: 0238
A `gen:status` region of the benchmarks page writes `![alt](gen/chart-WHAT.svg)` above its table
and the SVG into gen/ (crates/djs-docs/src/chart.rs), so regen keeps both current and check
fails on either being stale. Three shapes: grouped columns, paneled horizontal bars, ratios
around a parity line; log axes with 1-2-5 ticks thinned to fit. Colors: the diagrams'
stylesheet (svg::style_with) plus Elanor's data-viz palette for series (the same in light and
dark mode) and the positive/negative fills for ratios. Each SVG has `<title>` and `<desc>`
(aria-labelledby); the figure caption is the alt text; the exact numbers stay in the table.

## D3170: djs-embed: the native embedding crate for Rust hosts
date: 2026-11-07 · by: coordinator · issue: 0240
Rust hosts (first: Gondolin's daemon, which runs JS network hooks in-process) get a crate of
their own, `crates/djs-embed`, instead of reaching into `djs_core::vm`, `interp`, `heap` and
`wire`. It is a std crate in the workspace with its own version (0.x semver: a minor bump may
break, a patch may not); its public API names no djs-core type, so djs-core keeps changing in
place (D024) behind it. Its only dependencies are djs-core and djs-prelude (no third-party
runtime crates: hosts like Gondolin count every crate), and it builds wherever djs-core does
natively (darwin, linux-musl with a foreign linker). The shape: a compiled, `Send + Sync`
snapshot (an image of a VM after the host's setup scripts ran) that instantiates fresh VMs or
resets one in place; typed host functions (sync and async) with `&str`/`&[u8]`/number/bool/
plain-data arguments and results; an async entry point that ends with the settled value of the
returned promise; plain data in and out without JSON (the wire format, with a Rust encoder,
and an outbound encoder from VM values); fuel, memory and wall-clock budgets a script cannot
catch; one error type, and a documented failure model in which every failure fails closed and
an engine panic poisons only the VM it happened in. Hosts depend on it by path or by a git
revision until djs publishes crates.

## D3171: ES modules in djs-core, behind the `modules` feature
date: 2026-11-07 · by: coordinator · issue: 0241
djs gains the module goal (ARCH listed modules as a non-goal; embedders now need them: policy
modules importing host-provided `@sdk/...` libraries). Constraints the implementation keeps:
- **No I/O in the engine.** The host resolves specifiers and supplies sources through a loader
  (resolve `(specifier, referrer) -> key`, load `key -> source`), synchronously, before or
  outside runs; specifiers are opaque strings (no URL or path semantics built in). A dynamic
  `import()` becomes an async host call that the host answers with a loaded graph.
- **Stackless.** Module bodies run as ordinary frames; the evaluation algorithm that calls them
  is in the builtin language (the leaf rule), so evaluation suspends, meters fuel and survives
  images like any code.
- **Heap state.** Module records, environments and namespaces are cells reachable from the
  realm, so a snapshot taken after evaluation carries the evaluated graph (the
  compile-once/instantiate-many model).
- **Size.** The Rust side is behind a djs-core feature `modules` (cut by a constant like
  `FULL`, D2411), on in native default builds and off in every djs-wasm module, which stays
  within its budget (D020); without it the module goal is a SyntaxError and `import()` keeps
  D1582's rejection.
- **Tests.** test262's module tests (`flags: [module]`, language/module-code,
  language/import, dynamic-import) move from the skip group to active as the runner gains a
  fixture loader, with failures in the ratchet; the corpus oracle stays the codemode model.

## D3172: TextEncoder and TextDecoder (UTF-8) are builtins
date: 2026-11-07 · by: coordinator · issue: 0244
Embedders hand bytes to scripts (request bodies), and scripts that count or produce UTF-8 need
`TextEncoder`/`TextDecoder`; a host function per conversion costs a suspension each. They
become builtins of the full build (left out of the slim one), UTF-8 only as the WHATWG
Encoding standard's `TextEncoder` and the `utf-8` decoder (its labels, `fatal`, `ignoreBOM`,
`stream`), over leaf intrinsics that reuse the engine's transcoders. If their cost in the
shipped module goes over 1.5 KB gzip they are cut from it like a slim family instead, and
the size ceiling rises only by what is measured (recorded in the issue's size decision).
`structuredClone` is not part of this.

## D3173: No URL or URLSearchParams in djs
date: 2026-11-07 · by: coordinator · issue: -
A WHATWG URL parser needs IDNA mapping and punycode, tables of a size the shipped module
cannot carry (D020), and its one embedder so far (Gondolin) wants URL semantics identical to
its own Rust parser, which a host function gives exactly. So embedders that need URL provide
it from the host, in their own shim. Revisit when a second embedder needs it in-engine.

## D3174: libm becomes a djs crate (djs-libm)
date: 2026-11-07 · by: coordinator · issue: 0245
Embedders count third-party runtime crates (Gondolin has a budget), and libm was djs-core's
only one (D1150). The functions djs-core calls are copied from libm 0.2.16 (MIT, a port of
musl's libm) into `crates/djs-libm`, with its license and the `arch` paths (wasm32 and x86
intrinsics) kept, so every result stays bit for bit what it was; the version pin of D1750
becomes the crate's own sources in the build hash. Updating it is copying a newer libm's
files, with a decision.

## D3215: djs-libm: what was copied, and how the build hash covers it
date: 2026-11-07 · by: libm · issue: 0245
Implements D3174; supersedes D1150's dependency (its notes on `pow`'s special cases and on
qjs's last-bit differences still hold) and D1750's libm pin. `crates/djs-libm` holds libm 0.2.16's files for the functions djs-core
calls and everything they reach (`fma`, through cbrt; `scalbn`; the trig reductions; the
generic floor/ceil/trunc/sqrt/fmod/fabs and the `support` traits they use), each file as
in libm except for cuts: other functions and the f32/f16/f128 variants, libm's own tests,
items of cfgs off in its default build (`f16_enabled`, `f128_enabled`, `assert_no_panic`,
`unstable-public-internals`), trait items nothing reaches (`DFloat`/`HFloat`, the u256
narrowing division). `arch_enabled` (libm's default `arch` feature) is always on;
`intrinsics_enabled` and `x86_no_sse` are declared cfgs that are never set.
- **Arch routines.** Kept what libm selects by default: SSE2 `sqrt` and runtime-detected
  `fma` (FMA/FMA4, else the soft one) on x86, `fsqrt`/`fmadd` on aarch64. On wasm32, libm's
  default build uses the generic code: its wasm32 routines (`f64.floor`, `f64.sqrt`) need
  `unstable-intrinsics`, so they are not copied. The i586 routines (x86 without SSE2) are
  left out: djs does not build for it.
- **Style.** Edition 2024 like the workspace; `cargo fmt` changed only blank lines. Clippy
  runs with libm's allow-list plus `explicit_counter_loop` and `unstable_name_collisions`.
  The crate has libm's `unsafe` (`force_eval!`'s volatile read, the unchecked `i!` indexing
  of release builds, `asm!`, transmutes): it is libm's code, kept as is, so djs-core's
  no-`unsafe` rule does not apply to it.
- **Build hash.** djs-core's build.rs hashes djs-libm's `src/` with its own sources;
  `BUILD_INPUTS` lists the features only (the `libm X.Y.Z` entry and its Cargo.lock check
  are gone). Any edit to djs-libm changes `BUILD_HASH`.
- **Proof.** `crates/djs-libm/tests/same_as_libm.rs` compares every exported function with
  libm 0.2.16 (a dev-dependency) bit for bit on special values, branch thresholds and
  200,000 random inputs each (also run under x86_64 Rosetta, where the x86 paths are used).
  The shipped module is 174 bytes smaller (979,984 vs 980,158): with `rem_pio2f` gone,
  `rem_pio2_large` has one caller and LLVM inlines it; no other function changed size.
- djs-libm is opt-level 1 in dev, as libm was through `[profile.dev.package."*"]` (D1785).
## D3180: djs-core's `embed` feature; the wire format's uncounted UTF-8 strings
date: 2026-11-07 · by: embed · issue: 0240
What djs-embed needs from djs-core and the wasm modules do not is behind a new code-only
feature `embed` (in djs-core's defaults, image-neutral in build.rs and compat_inputs.rs, off in
every djs-wasm module): `wire::encode` (VM value → wire buffer, never runs JS, refuses with a
path like js/src/wire.ts), `Vm::encode_wire`/`decode_wire`, `Vm::reset_to`,
`ExecOptions::fatal_out_of_memory` (D3181) and the decoder's tag `0x12`, a UTF-8 string without
its UTF-16 unit count: the decoder counts units from the lead bytes (exact for valid UTF-8) and
decodes it as a counted `0x10`, which validates while it transcodes. The format stays version 1:
the tag is additive and every decoder is this repository's (D024); a decoder without `embed`
rejects it as an unknown tag, and js/src/wire.ts does not write it (V8 strings are UTF-16, it
knows the count). `wire::Writer`, the Rust encoder, is not gated: nothing in the modules calls
it. Why gate rather than measure: the shipped module had 14 B of gzip slack, and the module's
function order follows first references, so even code that is never called on a module path
moved its functions: an unused `Vm::decode_wire` moved `wire::decode`, and `transcode`'s
closures moved with a new enclosing function; each cost ~70 B gzip at the same raw size. With
the gating the shipped module is byte-for-byte the size it was (426,792 gzip, 980,158 raw) and
the slim one 360,471.

## D3181: Reaching the heap cap can end the run uncatchably
date: 2026-11-07 · by: embed · issue: 0240
`ExecOptions::fatal_out_of_memory` (builds with `embed`): the heap cap's out-of-memory fault
takes the host-out-of-memory path (D1690) instead of opening the reserve and throwing the
catchable `InternalError`: the run stops at the next instruction boundary with
`Status::HostOutOfMemory`, before any handler instruction runs (no `catch`, no `finally`, no
promise reaction), and `Vm::run` aborts the call, so the VM is idle with its realm intact.
djs-embed turns it on for every VM it makes; a policy cannot swallow its own memory exhaustion.
The status keeps its name; its doc says it covers both causes. Data the abandoned call held in
globals stays (a host resets the instance); the abandoned frames' data is garbage at the next
collection.

## D3182: `Vm::reset_to`: reload an image into the VM's own arena buffer
date: 2026-11-07 · by: embed · issue: 0240
`Vm::reset_to(&[u8])` (builds with `embed`) is `replace_image` from a borrowed image: it takes
the arena's `Vec` out, clears it, copies the image in and loads it as `from_bytes_trusted`
does, keeping the options, the dump caches and the derived Unicode composition table. One
copy, no allocation once the buffer is big enough (it keeps the capacity a call grew it to).
On an error the VM has no heap left (djs-embed poisons the instance). Copy-on-write images
were not attempted: the arena is one `Vec`, and the copy is a memcpy. Measured (release,
Apple M-series, load ~140, `cargo test --release -p djs-embed --test it bench -- --ignored`)
on a 192 KiB snapshot of a shim-sized setup: instantiate 2.9-3.9 µs, reset 2.8-3.0 µs, an empty
call 0.2 µs, a header policy (Headers class over a Map, allowlist, 2245 fuel) 12.5-13.5 µs
pooled, 17-18 µs with a reset or a fresh instance per call. The prototype measured 6-7 µs for
`from_bytes_trusted(image.clone())` of 238 KiB, whose exact-size buffer the heap reallocated
on its first growth; djs-embed's instances start in a buffer of the image's restore capacity.

## D3183: `Vm` is `Send` in every build
date: 2026-11-07 · by: embed · issue: 0240
An embedder's async task holds an instance across awaits and may resume on another thread,
so `Vm` must be `Send`. It was not in builds with `dump`: the dump caches (`Vm::base_names`,
`dump::Cache` and the maps they hold) used `Rc`. They use `Arc` now (vm/dump*.rs, a rename);
workspace builds unify `dump` into djs-core, so djs-embed's compile-time `Instance: Send`
assertion would otherwise fail there. `vm::tests::vm_is_send` keeps it so. Not `Sync`: the
caches are `RefCell`s, and one VM serves one call at a time.

## D3184: djs-embed's API and failure model
date: 2026-11-07 · by: embed · issue: 0240
The shape of crates/djs-embed 0.1 (its crate docs are the reference): `Snapshot::builder()`
defines host functions (`HostFn::sync`/`asynchronous`, by id and global name) and plain-data
globals first, then runs the setup `Source`s in order (a `SourceKind` leaves room for modules,
0243); setup may not call host functions (no host to answer: `Error::Usage`); builtins are
frozen by default; the image is sealed (everything old). `Snapshot` is an `Arc` (`Send + Sync`,
asserted), `Instance` is `Send` (asserted). A call is `start(path, args)` then `step()` →
`Step::{Done, HostCall(id), Wait(new ids), Yield}`; `this` is the object the dotted path ends
on; `Instance::call` is the same loop for hosts that answer at once. The entry's return value
is kept in a heap handle across runs (no settle callback in the script); when it is a promise
it is marked handled and the call settles with it; settling aborts the VM, dropping async
calls nobody awaits and queued jobs; the value stays readable (`result()`) until the next
`start`. Values in are `Value<'a>` (borrowed `&str`/`&[u8]`/`&Data`/wire; strings via
`alloc_str`, ASCII a memcpy; bytes one copy: the single arena cannot wrap host memory); values
out are `ValueRef<'a>` borrowing the instance (`as_str` borrowed when the string is flat
ASCII, `as_bytes` borrowed from a `Uint8Array`/`ArrayBuffer`), `to_wire` (D3180's encoder,
capped at the memory limit) and `to_data` (through the wire form). `Data` is the owned tree;
array holes are a `Data::Hole` variant (not `Option` elements: most arrays have none);
strings read out with lone surrogates get U+FFFD (the wire form keeps them). Failure model:
everything that is not a settled value is an `Error` and ends the call (abort: idle, realm
intact), except a stale call id given to `resolve`, which changes nothing. Budgets: fuel per
call in slices, a deadline checked before each slice (`deadline()` for hosts awaiting host
work), the memory cap uncatchable (D3181). The cap's catchable OOM error, should one escape
(a call's argument setup), maps to `Error::OutOfMemory` too. Every djs-core entry that runs
engine code goes through `catch_unwind`; a panic is `Error::Panicked` and poisons the instance
(every later use `Error::Poisoned`), never the snapshot or other instances: djs-core keeps no
global mutable state outside tests (checked: no `static mut`, atomics or thread-locals outside
cfg(test); the `OnceLock`s are in test modules), everything is in the VM's own heap. This needs
`panic = "unwind"`. Host-thrown errors (`Throw`) carry no stack (`Vm::new_error` records none).

## D3220: Base images are target independent; cross builds are a manual check
date: 2026-11-07 · by: libm · issue: 0246
djs-prelude's build script makes the base image with a build-machine djs-core and the target
embeds it; that is correct for every target, because the image has no target-dependent byte
(big-endian targets included in principle, though none is built or tested): heap cells are read and written with explicit
`from_le_bytes`/`to_le_bytes` (heap/layout.rs), offsets are `u32` (wasm32 modules have always
embedded a base made on a 64-bit build machine), and the floats computed while making it go
through djs-libm, whose arch routines (sqrt, fma) are correctly rounded, so x86 and aarch64
agree. So no target-side base build and no per-target image check in the build. Verified by
`scripts/cross-check.sh`: djs-embed's example for aarch64/x86_64-apple-darwin and
-unknown-linux-musl (zig cc as the musl linker, Gondolin's wrapper approach in
scripts/cross/) compares its embedded base with one it makes at run time and prints the same
lines on all four (run natively, under Rosetta, in docker); also as a consumer outside the
workspace by `git = "file://..."` and a rev, with a fat-LTO release profile. It is not a gate
leg: zig, the rustup targets and docker are not guaranteed on the gate machine; TESTING.md
"Cross builds of djs-embed" says when to run it. Hosts depend by path or a git rev; nothing
needs to move into the crate directories for that (cargo's git checkout is the whole
repository), only a crates.io package would. djs-embed's runtime crates: djs-embed, djs-core,
djs-prelude, djs-libm.
