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
