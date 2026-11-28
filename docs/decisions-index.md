# The decision log by topic

`docs/decisions.md` is append-only and in the order decisions were made; this index groups
them by topic. A decision appears where it matters most (a few under two topics). When
one refines or replaces another, the later one says so; the notable replacements are
listed at the end. Search the log for the ID (`## D1234:`) to read a decision.

## Process, tooling and the development flow

- D001: Work tracking in files, worktrees in `.worktrees/`
- D002: dquickts is read-only
- D005: Toolchain
- D010: djs-core at opt-level 1 in the dev profile
- D012: Development flow: per-agent job caps, iteration habits, worktree slots
- D1785: Dependencies at opt-level 1 in the dev profile
- D1793: Agents' runs are niced; the gate is not
- D1792: The gate runs in one persistent gate worktree
- D1791: Generated files are regenerated after a rebase, not merged; line lists union
- D1790: The gate's runners decide timeouts by fuel, not by the wall clock
- D1200: Tests that run subprocesses use generous timeouts
- D1344: djs-bgen's behavior test has a timeout
- D2100: The guide: Markdown in docs/guide, our own generator, generated parts checked in
- D2101: Status numbers are generated: counted from the repository or recorded with their commit
- D2102: The guide is self-contained; checked by djs-docs
- D2103: Live examples: a textarea over the highlighted code, our own JS tokenizer
- D2104: Every repository path in the guide is a source link, made by the renderer

## Testing: test262

- D003: test262 pinned to dquickts' commit
- D1020: test262 runner execution model
- D1021: qjs engine details
- D1022: test262 grouping rules
- D1023: Ratchet file semantics
- D1070: The djs engine in djs-test262
- D1071: Unimplemented syntax is not a SyntaxError for test262
- D1072: test262 in the gate
- D1593: test262's scan-eval group becomes direct-eval
- D1531: qjs test262 results are cached

## Testing: difftest corpus, stress modes, both backends

- D1100: Difftest compares what pi's codemode shows the model
- D1101: djs-difftest reuses djs-test262's library
- D1102: Importing dquickts' tests
- D1160: djs in difftest provides the codemode globals natively for now
- D1440: djs-difftest runs the shared prelude (supersedes D1160)
- D1161: Stress variants in difftest and the gate
- D1221: End-to-end corpus with qjs-generated expectations
- D1347: A stressed difftest run that only times out is reported, not failed
- D1530: difftest `no-stress`; a stressed timeout is a failure again
- D1540: D013 in practice: which overflows are RangeErrors; `djs-expect` in difftest

## Testing: determinism, journals and benchmarks

- D1720: Determinism checks in the gate: heap images across schedules and builds
- D1725: The determinism checks compare raw images too
- D1730: Canonical hash and run journals
- D1710: GC points and raw images do not depend on the fuel slice
- D1190: Benchmark harness
- D1191: Engine benchmarks run in the difftest codemode model
- D1192: Wasm size probe

## Target and code size (wasm first)

- D016: wasm32 is the primary target for performance and memory design
- D1232: The wasm profile uses opt-level "s", not "z"
- D1260: Inlined fixed-size arena accessors; the wasm profile stays at opt-level "s"
- D1570: The wasm module links with --compress-relocations
- D1571: The bytecode emit helpers are not inlined
- D1572: The compiler's internal error is boxed
- D1573: Arena out-of-bounds panics carry no offsets in release builds
- D1574: The interpreter's Fault is boxed
- D1575: The wasm profile strips the custom sections
- D1634: Code size: the native API is out of line in wasm
- D1780: A block ASCII check in wasm32
- D1432: No BTree collections and no core slice sorts in djs-core

## Values, heap layout and strings

- D1050: Value encoding
- D1051: Arena and cell header
- D1052: Layout description macro
- D1053: String representation
- D1054: Atom representation; symbols are atoms
- D007: Internal-reference Value tag
- D1055: Object storage
- D1830: Property entries are 16 bytes; accessors live in an AccessorPair cell
- D1831: Shapes with strong transitions and a per-heap shape budget
- D1261: The arena keeps a zeroed frontier; data is written once
- D1610: Append ropes: `s += x` writes into a growable buffer
- D1871: No Rust allocations in short concatenation and fromCharCode
- D1434: String constants are shared across a compilation unit

## Garbage collection, memory limits and out of memory

- D1090: Copying collector
- D1091: Handle table
- D1381: Map and Set are ordered hash tables rehashed by the collector
- D1382: WeakMap and WeakSet are ephemerons in the Cheney collector
- D1513: WeakRef and FinalizationRegistry on ephemeron tables; cleanup jobs after a collection
- D1870: Arena growth stops at the GC threshold; thresholds in 64 KiB pages; no to-space reuse
- D1600: Out-of-memory recovery: a collection at the handler and a reserve below the cap
- D1601: The GC threshold is part of the heap image
- D1690: Running out of host memory is not a JS out-of-memory error
- D1700: A failing allocation collects and retries its instruction once
- D1900: Native builtins collect and retry at the heap cap
- D1910: Dense-array loops, fromCharCode and the URI functions as `@fast`; entries decline at the heap cap

## Heap images, snapshots and the build hash

- D1056: Heap images are verified on load
- D1320: VM images carry a build hash (Roots::BUILD)
- D1321: The base snapshot (djs-prelude)
- D1330: The build hash includes a hash of djs-core's sources
- D1750: The build hash covers features and libm; canonical f32/f16 NaN (libm pin superseded by D3215)
- D1431: The base image is compacted before it is collected
- D1340: The prelude keeps no source text
- D1430: Privileged code keeps no line tables or local names
- D1433: Prelude closures carry no `length`, `name` or `prototype`
- D3700: Logical snapshots are djs's persistence format (with D3701-D3723)
- D3730: Continuation dumps are removed; snapshots are djs's only persistence format
- D3731: The REPL's `--debugger` makes `debugger;` a pause; `--frozen` only freezes the builtins
- D3732: The shipped module compiles eagerly
- D3733: Modules with snapshots keep the 128 KiB stack
- D3734: A Sandbox run's result is the entry call's outcome; states only for open calls

## Numbers, Unicode and text tables

- D1031: Unicode tables from UCD 17.0.0, packed ranges
- D1040: Number conversion algorithms and the shared power-of-five table
- D1041: Decimal strings are always correctly rounded (qjs truncates after 38 digits)
- D1262: Short decimals format without Ryu
- D1782: `short_decimal` gives up early on full-precision numbers
- D1114: Case mapping tables
- D1760: Unicode normalization, on by default

## Lexer and parser

- D1030: Lexer shape: context-free, span-only tokens, keyword ids
- D1032: Line table semantics
- D1080: Speculative parsing never emits bytecode
- D1360: destructuring patterns are parsed after their value (deferred patterns)
- D1820: The long tail: reference evaluation order and parameter scopes

## Compiler and bytecode

- D006: Accept compiler design note 0008 (docs/compiler.md)
- D1081: Name resolution runs once per compilation unit, children first
- D1082: Bytecode encoding and the instruction table
- D1083: Captured bindings live in heap Box cells
- D1084: Exception handler tables and gosub/ret for finally
- D1130: The bytecode table and emitter API
- D1131: Code cells
- D1140: The compile() entry point and template conventions
- D1141: Script-mode globals in bytecode
- D1142: P0 compiler scope and structure
- D1240: Function objects get name, length and prototype from their template
- D1241: Parameters and per-iteration bindings
- D1250: try/catch/finally compilation
- D1270: Intrinsic ids
- D1271: Object literal compilation
- D1292: for-in/of code layout and iterator close
- D1361: array destructuring keeps `next it` and reaches it with `pick`/`poke`
- D1362: template objects are created on first use and cached in the constant pool
- D1400: Private names are private symbols in the object's own property table
- D1402: Home objects are a closure slot after the captures
- D1403: Field initializers are two parked functions per class
- D1404: Derived constructors: `this` is a local, the result is checked after the body
- D1411: A return from a resumed `yield` goes through a throw region
- D1412: Generator/async state as an internal local, placeholders in phase 1
- D1580: `using` declarations keep a resource Record on the operand stack
- D1581: Script and eval completion values; `Goal::Eval` for indirect eval
- D1860: Loop rotation and peepholes in the compiler; inc_loc/dec_loc; fast_more
- D1862: No dense opcode range

## Interpreter: execution, fuel, fast paths and caches

- D1085: Re-entrant ops finish immediately, slow paths call privileged JS
- D1086: Frame records in the index-addressed value stack
- D1150: libm is the math dependency of djs-core (superseded by D3174, D3215)
- D1151: Interpreter state in the arena
- D1152: Host call protocol and fuel semantics
- D1280: Slow paths replace an operand and re-execute the instruction
- D1861: The Retry continuation
- D1350: an interpreter fast path for the hottest simple ops
- D1352: more fast paths: leaf intrinsics, floats, dense elements, own-property writes
- D1783: Fast paths for array appends, strict equality and insert2/insert3
- D1784: Bulk reads of dense arrays (fromCodePoint, apply)
- D1620: Inline caches: verified table indexes in a per-template array
- D1621: Prototype property caches: a receiver-prototype offset, an epoch, and GC clearing
- D1740: Fuel overshoot carries to the next run; a suspended intrinsic is charged once
- D1711: Regexp fuel can overdraw; backreferences are charged in full
- D1712: A suspended intrinsic's refund carries to the next run as a credit
- D013: Stack overflow is a RangeError
- D1282: Error stack traces
- D1375: Suspendable frames: frame slices in Generator objects, tail calls for await
- D1800: iter_next: iteration steps without result objects

## The builtin language (djs-bgen), native builtins and @fast

- D1170: djs-bgen is Rust with its own parser for the subset
- D1171: Builtin subset rules
- D1172: Generated prelude format and installation
- D1173: Reentrant intrinsics are single-step
- D1281: Prelude installation
- D1291: `%Name%` install targets in bgen
- D1310: Builtin installation forms
- D1311: Builtins are not constructors; bound functions
- D1315: Symbol keys and attributes in installation tags
- D1393: Builtin constructors do not read newTarget.prototype in the interpreter
- D1631: Native frames are ordinary frame records with a header bit
- D1632: The bgen Rust backend: states are basic blocks, calls go through call_intrinsic
- D1634: Code size: the native API is out of line in wasm
- D1803: bgen: try/finally and constructors in `@native`
- D1850: Leaf fast paths for everyday builtins (`@fast`)
- D1801: Dense-array leaves for the element loops of Array builtins
- D1910: Dense-array loops, fromCharCode and the URI functions as `@fast`; entries decline at the heap cap
- D1561: Bulk paths for apply, fromCodePoint and Math.max

## Codemode model, Script goal and globals

- D004: Script goal in addition to the async body
- D008: Codemode top level is a function body
- D009: Keep source text for Function.prototype.toString
- D1410: The codemode body is an async function
- D1141: Script-mode globals in bytecode
- D1220: Script-mode globals at run time
- D1581: Script and eval completion values; `Goal::Eval` for indirect eval

## Builtins: objects, functions, errors and conversions

- D1312: Error constructors and stacks
- D1313: Primitive wrappers and property lookup on primitives
- D1314: Math.random
- D1341: Array builtins follow the spec's Get, with leaf fast paths
- D1342: String builtins and their leaves
- D1343: instanceof through InstanceofOperator when it matters
- D1380: Reflect and construct with an explicit new.target
- D1401: Private elements need an extensible object (nonextensible-applies-to-private)
- D1441: Object.prototype.__proto__, and Function.prototype.toString of proxies
- D1451: `new` when reading new.target.prototype runs user code
- D1453: Primitive bases: [[Set]] through setters and proxies on the prototype chain
- D1542: Host display of errors never runs JS: accessor names fall back to the native name
- D1840: Function.prototype is a builtin closure; ArraySetLength in [[Set]]; native DOMException, atob, btoa

## Builtins: iteration, generators and async

- D1290: The iteration protocol lives in the builtin language
- D1345: Builtins over iterables
- D1346: The String iterator reuses Array Iterator objects
- D1351: array spread: original iteration in Rust, everything else via %ArraySpread
- D1363: object spread with getters goes through %CopyDataProperties
- D1420: Function kinds: [[Prototype]] and `prototype` from the ASYNC/GENERATOR bits
- D1421: Async generators and async iteration on the generator ops
- D1422: `yield_star` passes the inner result object through
- D011: Promote generators and async iteration to active
- D1510: Iterator helpers are slot objects with a hand-written state machine
- D1511: Array.fromAsync is a state machine resumed by builtin closures
- D1802: Iterable consumers step without result objects
- D1512: Disposable stacks: one slot class, resources as triples, async disposal as a state machine

## Builtins: promises, jobs, host calls and the inbox

- D1370: The job queue, and when `run` reports Done
- D1371: Promises: Rust slots and reactions, the algorithms in builtins/promise.ts
- D1372: Builtin closures and internal records
- D1373: Unhandled rejections are tracked in the heap and taken by the embedder
- D1374: Async host functions: a promise plus an entry in the pending host call table
- D1423: Unhandled rejections reach the host; reading a result promise handles it
- D1592: queueMicrotask is native; a throwing callback is reported like a rejection
- D1670: Async host results are macrotasks: the host inbox
- D1152: Host call protocol and fuel semantics
- D1642: `Vm::run_metered` and `Vm::backtrace`

## Builtins: Proxy, Reflect, BigInt, typed arrays, collections

- D1450: Proxies: an empty heap cell, traps in builtins/proxy.ts, same-signature redirects
- D1452: for-in through proxies is lazy, in qjs's trap order
- D1460: BigInt values are always heap cells, compared by value
- D1461: Arbitrary-precision arithmetic in `crate::bigint`, capped at 2^20 bits
- D1462: BigInt operators are a slow path beside `exec`
- D1463: BigInt typed array elements: placeholders in `&self` reads, `Elem` writes
- D1390: ArrayBuffers and typed arrays in the heap
- D1391: %TypedArray% and the realm's constructors table
- D1392: Bytes on the boundary are Uint8Arrays, read back as copies
- D1381: Map and Set are ordered hash tables rehashed by the collector
- D1382: WeakMap and WeakSet are ephemerons in the Cheney collector
- D1513: WeakRef and FinalizationRegistry on ephemeron tables; cleanup jobs after a collection

## Builtins: JSON, RegExp, Date, URI, Unicode

- D1120: Native JSON leaf (parse and stringify fast path)
- D1331: JSON builtins: leaf parse and fast stringify, the rest in the builtin language
- D1611: JSON.parse source text access
- D1901: JSON.stringify: plain subtrees and Dates in the fast path
- D1110: Annex B regex grammar in non-unicode mode
- D1111: Regex engine design
- D1113: Where the regex parser deliberately differs from qjs
- D1332: RegExp objects, suspendable matching and regex literals
- D1500: Unicode property escapes, on by default
- D1501: Compact loops over one character (STAR)
- D1502: Regex literals compile once
- D1503: Fast paths for unmodified RegExps
- D1504: Undoing a log entry restores the slot's generation
- D1560: RegExp exec and test without protocol overhead
- D1562: A tight forward loop for STAR
- D1563: Follow sets that accept the end of the input; disjoint loops
- D1383: Date reads the host's clock and a fixed local offset
- D1541: URI functions and escape/unescape as leaf intrinsics, with QuickJS's messages

## Dynamic code, eval and modules

- D014: Indirect eval and the Function constructors, no direct eval
- D015: Sloppy `this` binding for Function-constructor functions
- D1630: D015 as a template flag, bound on the slow call path
- D1590: CreateDynamicFunction parses one function and checks where its parts end
- D1591: Dynamic code compiles in a leaf, runs as an ordinary call; fuel per code unit
- D1582: `import()` without modules: compiled, rejected in a job
- D1485: strip-types parses module syntax; djs still has no modules

## TypeScript and decorators

- D1480: TypeScript is a compile flag; the skipper ports dquickts' rules
- D1481: Erased ranges are recorded by committing code, blanked like ts-blank-space
- D1482: Enums and namespaces are closures; their members resolve after parsing
- D1483: const enums are folded at resolution
- D1484: Parameter properties follow useDefineForClassFields
- D1485: strip-types parses module syntax; djs still has no modules
- D1650: Decorators after qjs, the runtime in builtins/decorators.ts

## Embedding: wire format, reader, wasm ABI, determinism

- D1180: Inbound wire format, version 1
- D1181: The gate runs node
- D1781: Wire decode validates UTF-8 while transcoding
- D1210: Exported heap layout drives the host-side reader
- D1211: Outbound reads are trees; strings via node's Buffer
- D1230: The djs-wasm ABI: integer exports, staging/out buffers, numbered handles
- D1231: djs-wasm uses std and its built-in dlmalloc
- D1491: djs-wasm exports for embedders: file names, random seed, build hash
- D1680: Deterministic mode: weak references collected, strong or absent; the Sandbox's deterministic option
- D1642: `Vm::run_metered` and `Vm::backtrace`

## CLI and REPL

- D1640: The REPL's line editor is hand-rolled, on `stty`
- D1641: REPL inputs: expressions, Scripts, and a rewrite for top-level await
## Replaced or refined decisions

- D1160 (djs provides the codemode globals natively in difftest) is replaced by D1440
  (difftest runs the shared prelude).
- D1112 (no `\p{...}`) is replaced by D1500 (Unicode property escapes, on by default).
- D1347 (a stressed run that only times out is reported, not failed) is replaced by D1530.
- D013 (stack overflow is a RangeError) is spelled out in practice by D1540.
- D015 (sloppy `this` for Function-constructor functions) is implemented as D1630.
- D1850's rule that out of memory propagates from a `@fast` entry is changed by D1910
  (entries decline at the heap cap).
- D1700's collect-and-retry is extended to native builtins by D1900 and to `@fast`
  entries by D1910.
- D1610 (append ropes) and D1871 (no Rust allocations in short concatenation) both shape
  string concatenation; D1053 is the base representation.
- D1055 (object storage) is refined by D1830 (16-byte property entries, accessor pairs)
  and D1831 (shapes).
- D1783 appears twice in the log with two titles (fast paths for appends, strict equality
  and insert2/3; `put_elem` appends): both are the same decision.
- D2520 (`unsafe-heap` optional, not shipped) is amended by D3100 (the shipped wasm
  module builds with it; native and test builds keep the checks).
- Every continuation-dump decision (D2352-D3660, listed in D3730) is superseded by D3730:
  dumps are removed and runs persist as snapshots (D3700, D3720). The index lists none of
  them; those that only part of D3730 replaced (D2561, D2770, D2971, D3050, D3600) stay.
- The pi adapter (js/pi: D1490-D1495, D1660, D1890, D2650-D2653, D2772) and the compile
  cache (D3040) were removed by D3640; the slim build (D025, D2411-D2413) by D3642; the
  image dependency descriptor (D026, D2265-D2268) by D3641; lazy compilation (D2990,
  D2991, D2995) by D3740; Tasks and explicit bindings (D2771, D2972) by D3745; legacy
  decorators (D1651) by D3750. Those entries are gone from the index.

## Not yet sorted by topic

Decisions recorded since the index was last sorted, in log order. Move them into their
topics when the index is next reorganized.

- D1920: Compiler throughput and size in wasm (0092)
- D1832: Addition caches and receiver-shape proofs; 5-word cache slots
- D017: Note on duplicated decision id D1783
- D1940: Where a pi VM's baseline wasm memory goes
- D1941: A 256 KiB wasm stack; the parser's deep paths fit it
- D1942: Restored arenas reserve their target up front; `djs_image_staging`
- D1930: The tier-0 loop keeps the interpreter state in locals
- D1931: Calls and returns in the tier-0 loop: the same frames, block access
- D1960: Phase-1 code as packed instructions; identifier atom cache (0096)
- D1970: Immutable ArrayBuffers
- D1971: Error.prototype.stack is an accessor
- D1972: Promise.allKeyed and Promise.allSettledKeyed
- D1990: A restored arena reserves its GC threshold plus a slack
- D1991: `verify` keeps cell starts in a bitmap
- D2010: Error.captureStackTrace
- D2011: REPL completion reads the heap; `.step` pauses between fuel slices
- D1980: Privileged templates without debug fields (optional trailing fields)
- D1981: The atom index stays at load ≤ 1/2 (measured)
- D2020: `@fast` builtins are not also `@native`, except Array.prototype.push
- D2021: The number parsers have one instance; two-byte input is narrowed
- D2030: Error.stackTraceLimit as in qjs
- D2031: Call positions follow qjs: tail calls at the `return`, `new` after the callee
- D2032: `difftest: stacks` compares stack frames
- D2000: Typed locals of native builtins live in Rust variables, written through
- D2050: Error.prepareStackTrace and CallSite objects, as qjs has them
- D2051: SyntaxErrors from dynamic code start their stack at `<input>:line:col`
- D2052: A constructor's `prototype` is its last own property
- D2070: NamedEvaluation names only a function that is the whole initializer
- D2071: Syntax messages at the end of input and in dynamic functions follow qjs
- D2072: Constructor statics and namespace members in qjs's order
- D2060: Natives call plain closures by writing the frame in place
- D2150: Prototype objects in qjs's property order
- D2090: Fused dense HasProperty + Get in natives; one-pass string search and in-arena joins
- D2200: The guide's site uses Elanor, vendored from its published packages
- D018: Adopt mark-compact for in-VM collections
- D019: Snapshots: compact images now
- D020: The size budget is gzip relative to quickjs-wasi
- D021: pi integration deferred; codemode corpus harvested with redaction
- D2040: In-place mark-compact for in-VM collections; Cheney for canonical layouts
- D2280: The harvested codemode corpus
- D2281: Redaction of harvested scripts: arguments scrubbed, unknown words renamed
- D2250: JSON without per-key allocations, escapes by runs; two-byte filter in one-byte search
- D2263: Sandbox.bytes() collects first
- D2282: Difftest ratchet reasons are an excerpt around the first difference
- D2283: Harvested cases with structured tool results
- D2284: Extra redaction for structured-result harvests
- D2041: An old region that minor collections neither trace nor move
- D2042: Full collections do not promote; only seal points make cells old
- D2044: No card table and write barrier (old-generation phase 2) for now
- D2251: The base image is ordered by cell kind
- D2252: The wasm module ships the base image coded; hosts decode it once per module
- D2331: The shipped module is built with remapped source paths
- D2332: djs-wasm is no_std on wasm32, with a panic handler that never formats
- D2333: The shipped module drops panic locations (-Zlocation-detail=none, the one unstable flag)
- D3090: The shipped module builds core and alloc itself, with panics as bare traps
- D3091: Intrinsic names and signatures in compact run-time tables
- D3092: Opcode names apart from the instruction table
- D3093: The compiler names intrinsics at compile time
- D3094: Wasm builds run the prelude only with djs-core's `prelude` feature
- D3095: The run time reads a cell kind's minimum size, not its layout
- D2253: The base image drops the prelude's internal names
- D2254: `@native` only where benchmarks or real codemode pay
- D2045: The first collection after a canonical image is full
- D2046: No aging promotion
- D2047: Arena growth policies that did not help
- D2048: Peak memory is compared as a distribution over perturbed runs
- D2334: Unicode property tables coded by structure
- D2335: Only NFC keeps quick-check tables; the other forms' checks follow from the decompositions
- D2255: The shipped module inlines less (`-inline-threshold=40`), with hot helpers marked
- D2256: The URI functions and String.prototype.substr run their builtins (no @fast)
- D2336: Debug details of internal failures only outside the shipped module
- D2337: The decomposition stream in two parts with per-position prediction
- D2338: Primary composites are derived at run time and cached per heap
- D2053: A page allocator under dlmalloc in djs-wasm: huge blocks grow in place
- D2054: VMs from the base keep their initial arena room; json's +0.6 MiB is accepted
- D2339: Messages and formatted strings without core::fmt
- D2264: `djs heap` analyzes dumps through djs-core's heap code; djs-cli reads and writes snapshot containers
- D2055: The old-region walk uses per-kind slot plans
- D2056: The collection threshold stays at twice the live size
- D2340: Heap views show the program's own cells apart from the base realm
- D2340: Heap views show the program's own cells apart from the base realm; REPL dumps are containers
- D2300: JSON strings: quoting jumps to the first byte to escape; parsing skips plain words after an escape
- D2301: slice, join, split and CreateDataPropertyOrThrow on plain arrays in one step
- D2302: Size trade: +1.4 KB gzip for pi-like −12.7%
- D2057: The cells a VM replaces first sit at the end of base and sealed images
- D2058: The size ceiling rises by 600 B for stable snapshot layouts
- D2060: The base is decoded or injected straight into the first VM's arena
- D2303: Callback loops in one intrinsic, with a reused frame (`%EachDense`, the `Each` continuation)
- D2350: The portable-snapshot prototype lives outside the workspace and builds against older trees
- D2351: A snapshot importer never holds cell offsets across a VM run
- D022: Frozen builtins as a runtime VM option, plus an immutable builtin prefix
- D2062: `Roots::BASE_END` marks the builtin prefix
- D2371: The builtin registry: stable ids by spec path, append-only, gate-checked
- D2063: The base image ends with a tail of per-VM cells; nothing writes below BASE_END
- D2304: Frozen builtins: gates in the property layer, the override fix in [[Set]]
- D2064: Collections never move or free the builtin prefix
- D2065: A debug write barrier below BASE_END, armed by frozen builtins or one option
- D023: Notes superseding parts of D2304 after 0147
- D024: No compatibility with unreleased formats
- D2067: Builtin result shapes and the engine's queues are in the base; the ceiling rises 720 B
- D2410: The harvester reads p2 session databases from a read-only copy
- D2430: The base image stores each flat string once
- D2431: A two-level atom table: the base's atoms in the prefix, a VM's own in the tail
- D2432: `djs heap` shows the session's own part by default
- D2440: Prelude functions live in a realm table, not in captured Boxes
- D2305: Keys iterators step over holes; `%IteratorFastDrain` takes keys and values runs in one step
- D2306: Array.from(arrayLike, mapfn) runs through `%EachDense`
- D2307: Callback builtins on short arrays: no `%EachDense` for an empty array, and its call outside `step`
- D2510: No wasm JIT; ahead-of-time wasm for builtins stays `@native`
- D2441: The hot loop's dispatch keeps no per-instruction values across calls
- D2442: GC growth counts the builtin prefix as a fixed allowance
- D2308: Images hold no dead value-stack slots; embedder collections shrink a grown stack
- D2309: instr.py patches by checked anchors and splits user code from bytecode builtins
- D2443: Short opcode forms and shortened jumps, a recorded trade
- D2520: Arena access: one u64 compare by default, an optional unchecked `unsafe-heap` build
- D3100: The shipped and slim wasm modules read the arena unchecked (amends D2520)
- D2550: Fuel stays per instruction; charging only at safepoints measured, not adopted
- D2551: Large ASCII host strings are copied straight into a reserved heap string
- D2444: Code inline in the template, the rest of a user function in FuncInfo
- D2310: A twin stays only where it pays its bytes
- D2521: Vec pushes out of line outside the hot loops (`PushOut::push_out`)
- D2445: A closure's length and name come from its template until changed
- D2540: A 128 KiB wasm stack; parser nesting limit 550
- D2541: Instance memory: the arena kept at memory's end, lean start capacity, the coded base outside memory
- D2522: `Vec::with_capacity` out of line (`buf::vec_with_capacity`)
- D2552: reduce by argument count, its accumulator in the dense loop's state
- D2542: Slices: substrings that share a one-byte parent, copied out when they pin it
- D2543: A wasm stack overflow is reported as one
- D2544: GC thresholds count their pages from the prefix's end
- D2545: Nesting limits re-measured: parser 550 stays, regexp 212
- D2553: JSON.stringify builds a large result in its heap string
- D2554: Array species and ArrayCreate shortcuts
- D2446: One constant pool per compilation unit
- D2447: The base's atoms are renumbered without gaps
- D2448: A closure's template in its ELEMENTS field
- D2449: No direct native binding for thin builtins (L1, L2)
- D2450: The base's property table indexes at a load of three quarters
- D2561: Dumps are taken only at idle points
- D2562: In-flight builtin state is re-enacted; a few privileged loader primitives
- D2565: djs builds pi's future prelude shape in its own adapter first
- D2590: Corpus tool calls are idle points: `host.later`
- D2600: The CLI's `tools`: a Proxy that makes one async host function per name
- D2601: Scripted tool answers: by arguments, else in order
- D2570: Suspension maps from a role stack the parser keeps
- D2585: A load's open calls, handles and call ids
- D2602: The CLI's host globals `later` and `print`
- D2610: The exporter writes what is in flight as the calls that make it
- D2611: Variables of one name in several scopes get nested factories
- D2612: Re-enactments of iterators and for-await state
- D2630: Host-driven Sandbox runs, the first piece of the Sandbox run API
- D2632: Dotted host function names make a namespace object
- D2620: A codemode body's frame is an async function's
- D2634: `run`, `runAsync` and `replay` are built on the host-driven run
- D2621: A class's hidden bindings in a waiting frame are dead; private names are the class's
- D2652: prelude-next has no lockdown; the next mode has frozen builtins
- D2700: Heap images are no longer a persistence format; what was removed
- D2702: The Sandbox's `maxFuel` counts fuel across sync host calls
- D2201: The guide's site follows Elanor d3184572: its retuned type scale
- D2660: The gate is legs and builds in scripts/gate.py
- D2661: Every build configuration builds at once in its own target directory
- D2662: Gate legs are cached by input hash
- D2663: Three input classes, guarded
- D2664: `--full` and full-only legs; a full run daily
- D2665: The gate runs new executables once while the build goes on
- D2667: difftest runs stress variants as separate tasks, longest first
- D2269: No `djs heap` command and no `.image`; the REPL's `.heap` keeps the heap report
- D2666: The gate runs outside the merge lock; only the fast-forward is inside
- D2668: Clippy on every feature combination that ships
- D2669: Full runs in their own worktree
- D2720: Suspended frames become named-slot state machines in the compiler (SM)
- D2721: Slot names are construct@span; states keep kind#N
- D2722: Open host calls carry host JSON state; save/resume hooks; the Sandbox keeps the bookkeeping
- D2730: Slot names: offsets from the function's start of the element or the construct; pins list them
- D2731: Slots name every frame index; the operand stack stays in memory (refines D2720)
- D2790: No doctests, no empty test binaries, no bgen program in the gate; guarded
- D2791: One integration-test binary per package
- D2792: Builds end when cargo does; first runs overlap the legs
- D2732: Construct temporaries: statement state in named locals, listed where live
- D2733: Finally completions in temporaries; gosub/ret and return-address indexes gone
- D2734: No role vocabulary: stack values are named by construct; class sites refused by a marker
- D2800: String and number constants are not deduplicated per function while parsing
- D2832: Open host calls and Math.random's state are restored through host functions
- D2850: Suspension maps carry the syntax around each site
- D2851: Site syntax: member targets split, recording only in async functions and generators, pinned
- D2894: The CLI's open calls carry their origin and the answers file's counts as state
- D2852: Destructuring patterns are site-syntax nodes
- D2853: TypeScript expressions are transparent on site paths
- D2896: The gate clears a corrupt incremental cache and runs the job once more
- D2860: The plain exporter writes format 2's lines and an IR, and rewrites them
- D2870: Origin tracking reads a continuation's frame as its origin's
- D2871: Plain continuations of methods that use `super` are methods of the home's prototype
- D2872: `for await` in plain continuations: a loop over the saved record, the iteration in flight before it
- D2873: Origin tracking inside a continuation's rewritten statement, case tests, finally by jumps
- D2874: Destructuring around a continuation's site is written as a pattern again
- D2875: `yield*` in plain continuations: a `yield*` over a wrapper of the inner record
- D2876: `using` in plain continuations: the resources added again, a disposal as a scope of K
- D2880: Sites in loop heads, `var` declarations, catch parameters and more literals
- D2960: Exception handlers in unreachable code are dropped
- D2907: A continuation's callback runs in parts, at the ends of the blocks it needs
- D2921: The `jump_if_loading` opcode is removed
- D2908: A for-of or for-in goes on after the rest of its iteration, where nothing tells
- D2909: A continuation is a function called with the frame's values
- D2910: A continuation reads a value of the callback only after the part that makes it
- D2911: A continuation leaves out the frame's bindings nothing in it reads
- D2912: A continuation writes origin records only where it can stop again, and less text
- D2970: A host function is identified by a host-chosen string, one object per identity
- D2971: Dumps refer to host values by `$djs.host(name)`, never by where they are bound
- D2973: pi's next mode names its host functions `bridge` and `call`
- D2913: A continuation's record makes its layout again: `{f, at, h, r}`
- D2914: Literal operands, object literals, left operands and substitutions are read again
- D2915: Positional origin records; a generator's prototype is its function's
- D2917: Loops go on as written: builtin iterators as they are, `while` and `for` after the rest
- D2918: Assignments around the site are written as the original's
- D2919: String temporaries are literals again; a number substitution is read again
- D3000: A placeholder loop's first iteration is a scope of its own for liveness
- D2992: A Script keeps its source as UTF-8 bytes
- D3001: Values used once are written where they are used
- D3021: Array.fromAsync in flight is written as the runtime's `fromAsync`
- D3022: Amends D2896: the gate also cleans djs-core and djs-prelude after incremental corruption
- D3023: pi's adapter reuses disposed wasm instances, made as new before reuse
- D3002: A continuation's site takes its value from its callback; effectful values in a run
- D3003: Closures and continuations a continuation's blocks make are written where they are used
- D3024: A waiting frame's `this`, `arguments` and `new.target` that its arrows share are K's
- D3004: A reopened host call read once waits for its reader
- D3005: A hole the reader finds by reading the value is left out of the record
- D3006: Arguments in place outside K, functions movable, values nothing reads dropped
- D2841: Settled by the owner on 0216's review
- D3007: Host calls go into the callback part that reads them all; a factory's continuation takes its site's value
- D3008: How D2837–D2840 are carried out
- D3050: `debugger;` can be a pause: an await of the host's `debugger`, in dump builds only
- D3051: The guide cites no decisions, tells no history and names pi on one page
- D3060: `debugger;` pauses in the CLI: `djs repl --frozen` and `--debugger OUT`
- D3026: Late bindings and subclasses wait for the block's own declarations; keys keep lone surrogates
- D3052: goals.md holds every goal; links to a page's section must exist
- D3053: live examples pause at `debugger;` in their normal Run
- D3054: where a run waits: `Vm::call_site`, `vm_call_site`, `Sandbox.callSite`
- D3027: An object literal's method that uses `super` is made where its home is
- D3070: Measured figures in prose are inline status numbers
- D3080: Arena::truncate moves the zeroed frontier back
- D3081: Regexp follow sets only where a STAR uses them; allocation ceilings
- D3082: Slot plans drive tracing; the compactor updates and slides in one walk
- D3083: Benchmarks compare per collection, and the shipped module's instantiation
- D3084: Instruction ceilings for work the gate cannot time
- D2842: The npm package is @earendil-works/djs, version 0.1.0; dumps carry no version
- D3105: The npm package is built by tsc into target/npm/package and packed from there
- D3106: Each entry point is the whole API with its own module; Sandbox.defaultModule
- D3107: What the package exports and ships
- D3108: The publish workflow tests the one packed tarball everywhere, then publishes on a tag
- D3109: The gate packs the package and runs its smoke test in node
- D2843: The MIT copyright holder is Earendil Inc.; npm releases use trusted publishing
- D3110: The engine comparison runs every engine in wasm in one node process, a fresh VM per run
- D3111: The QuickJS releases are built with wasi-sdk by one script, as a minimal reactor
- D3112: monty runs as its wasm component, instantiated on the calling thread
- D3113: The codemode workload set is synthetic; the sessions it was derived from stay private
- D3114: The benchmarks page's tables are generated from `engines/*` status keys
- D3140: Below the GC threshold the arena grows by an eighth, in whole pages
- D3141: A VM's arena starts at the image in whole pages; the page allocator's HUGE is 128 KiB
- D3142: Big staging requests get their own block, placed after room for the arena
- D3143: The mark stack reserves 16 KiB up front
- D3144: Large live leaf cells count for 40% of the next collection's growth
- D3120: Pure leaf intrinsics run in the tier-0 loop
- D3121: djs-bgen inlines small helpers into the bytecode prelude
- D3122: Math.max / Math.min get @fast entries again
- D3123: The size ceilings rise for 0230
- D3135: The RegExp search is the matcher's; programs are read in place; one exec state serves many matches
- D3136: RegExpCreate shares a RegExp's program; pristine RegExps skip the flag getters and matchAll's Construct
- D3137: %RegExpExecOwn: RegExpExec without its lookups when they read only data
- D3138: A RegExp compile cache of 8 entries in a realm slot
- D3139: Size of 0233; the size ceiling rises by what it measured
- D3124: @fast entries for the string builtins still over 2x QuickJS after D3120/D3121
- D3130: await and async return subscribe and settle in the op where no user code runs
- D3131: Promise combinators do plain elements in Rust
- D3132: %RealmIntrinsic reads its name in place
- D3133: Size of 0232; the size ceiling rises by what it measured
- D3125: The comparator sort merges in leaves; only the comparisons run in the state machine
- D3125: The comparator sort merges in leaves; the default sort compares strings in place
- D3127: Map and Set get/has/set/add are @fast entries
- D3128: `+` of a string and a primitive, and `arr.push(x)`, finish in the hot loop's decline path
- D3129: Array.from from %Array% skips Construct; join reads slices in place
- D3126: Amends D3022: any undefined symbol from our crates' objects is incremental corruption
- D3151: A subclass outside its base's factory names the base by its location
- D3152: plain-node reads node's "Invalid time value" as djs's "Date value is NaN"
- D3155: Builtin closures keep `length` and `name` lazy
- D3156: @fast entries for the resolving functions, then, Promise.resolve and GetPrototypeFromConstructor
- D3157: An async function's state object is made at its first await
- D3158: allSettled and any element jobs settle in Rust
- D3159: Size of 0232b; the size ceiling rises by what it measured
- D3160: The construct path's instructions finish outside `step`
- D3161: A construct's frame holds its new.target from the entry safepoint on
- D3162: %ConstructorPrototype; constructor allocations are pure leaves
- D3163: `new` entries for Array, Map, Set and Date
- D3164: Size of 0236; the size ceiling rises by what it measured
- D3165: The bridge benchmark: each engine's best path, one operation timed
- D3166: Charts are generated with their table, from status.tsv
- D3260: The leaf backend: `@rust` builtins become straight-line Rust, user code at conversion sites
- D3261: One implementation per builtin; the bytecode backend option is gone
- D3262: The bytecode prelude's inlining budget drops to 40 characters
- D3263: Size of 0251; the ceilings rise
- D3264: Generated builtins call their conversions out of line
- D3265: A separate call instruction for builtins: measured, not adopted
- D3266: Size of 0252; the ceilings drop
- D3267: Number formatting and small allocations take short paths
- D3270: A new hash key rewrites only what lies above the builtin prefix
- D3271: The injected base is checked for identity, not hashed, and not zeroed first
- D3272: Fresh arena pages are zeroed as before: measured, not adopted
- D3273: The JS host draws seeds from an entropy pool
- D3280: An elastic shadow stack: 64 KiB linked, a larger block for deep compiles
- D3281: dlmalloc's first segment starts in the static data's last page
- D3282: Size of 0257; the ceiling rises
- D3283: Size offsets for 0257; the ceiling drops back
- D3320: djs-wasm exports one function, `djs(op, a, b, c, d, e, f, t)`
- D3321: Sandbox setup: the elastic wrapper in place, short names encoded in place
- D3322: A rope compares to a flat string chunk by chunk
- D3310: Byte kernels share one SWAR module; JSON strings scan and decode a word at a time
- D3311: Size offsets in JSON for 0260
- D3340: The intrinsics' names only where something reads them
- D3312: The held byte kernels of 0260; the ceiling rises 12 B
- D3313: D3312's ceiling raise undone
- D3341: The stack checker walks in code order
- D3342: A bump zone for compile allocations: measured, not adopted
- D3343: String literals without copies
- D3330: A comparator sort runs its comparisons in one reused frame (EachDense's sort mode)
- D3331: Object.fromEntries of an Array of entry arrays in one leaf
- D3332: Object spread into a fresh literal shares the source's shape
- D3360: An instance drops the coded base when its first VM takes the decoded one
- D3361: The compile zone after all (supersedes D3342)
- D3290: The ROOT_PREFIX_WRITTEN latch; collections skip an unwritten builtin prefix
- D3291: Past the GC target the arena grows by half its overshoot
- D3362: dlmalloc's first segment has a floor
- D3350: The Unicode tables leave the shipped module's memory; the host hands them over
- D3300: Known builtins: `Math.floor`, `String`, `Number.isInteger` without the lookup
- D3301: Builtin calls: what was measured and not adopted (0259)
- D3292: No allowance retune and no survivor-only string discount for now
- D3293: Growth past 1 MiB counts half
- D3294: A fixed nursery without a barrier: measured, not built
- D3370: new Error without four frames and two trips through `step`; the trace from heap strings
- D3363: The collector's side tables in the arena's tail: measured, not adopted
- D3364: The collector's side tables in the arena's tail after all (supersedes D3363)
- D3365: Array.prototype.join's two-byte result straight into the arena: measured, not adopted
- D3366: dlmalloc's floor drops to 87 KiB
- D3367: The gate checks the codemode rows' memory
- D3390: The tier-0 loop checks the end of the code where the frame changes
- D3391: Why edits to run_hot move the whole loop, and how to measure it (0268)
- D3368: A compile's peak memory: the parse's tables go before the resolver, the
- D3400: Joins and substrings copy rope leaves straight into the result
- D3401: @fast replace/replaceAll; slice views from one kind read
- D3369: The stack checker reads an op's table entry once and does not queue fallthroughs
- D3380: A regexp literal run is one node
- D3402: String-key element access without interning or leaving the leaf
- D3403: substr as a generated leaf; short substrings of slices copy from the parent
- D3430: EachDense's per-element step reads its state once; reduce's box at the end
- D3420: The base image's global table keeps room for 16 globals
- D3404: Rounding the arena's growth step down waits for the smaller base
- D3405: Base-atom keys stay on the base hash
- D3392: Arm-local costs in run_hot; a second loop around it measured and not adopted (0268)
- D3393: Property and element hits are read from the tier-0 loop's slice; string keys through the heap's helpers; the ceilings rise
- D3431: The callback's return inside the tier-0 loop: measured, not landed
- D3406: Ropes of many small leaves are flattened by the first join or substring
- D3440: test262 under GC stress is a full-only gate leg
- D3450: pi's prelude image key names the prelude by a number
- D3451: djs_image_staging leaves the bytes the host writes to the host
- D3452: A pooled instance leaves the image's bytes to the image
- D3394: The general path: where its cost is, template literals outside `step`, call_intrinsic 8 KB smaller
- D3410: Class constructors leave the tier-0 loop once per construct (parked)
- D3411: Map and Set iteration without a result object per value; new Set/Map of an array in Rust
- D3412: Date.parse reads the string's flat view; one wire transcode
- D3480: localeCompare of one-byte strings by bytes, as a generated leaf
- D3481: `a + b` of two short flat one-byte strings copies twice, without the walk
- D3482: One helper for one-byte copies of contiguous strings
- D3460: A pristine RegExp from the builtins latch; String.prototype.match/replace call the leaves
- D3461: The matcher's memory kept between matches: measured, not landed
- D3462: create_data_property's quick insert takes arrays too
- D3463: JSON.parse's parser starts with room for a small document
- D3490: for-in reads only enumerable keys and interns nothing per step
- D3491: Object rest without the builtin loop (parked)
- D3470: Cache kinds for absent properties and for accessors; a getter is called from the loop's exit
- D3471: `exec` and `json_regexp_intrinsic` read their operands once
- D3472: `typeof`, string comparisons and string equality without `step`
- D3500: status records state the load they were taken at
- D3510: The size budget gets 2 KB more (owner)
- D3520: The matcher's memory kept between matches: not landed (a new Heap field costs ~300 B gz)
- D3521: A new RegExp's lastIndex is added directly
- D3511: The construct path lands (D3410)
- D3512: Object rest lands (D3491)
- D3530: Fast entries for builtin getters, run inside the loop; typed arrays' named keys in the caches
- D3531: The loop off and the loop on leave the same bytes for string keys and getters; a test that stops after every instruction
- D3540: A fast keyed hash instead of SipHash-1-3 (owner)
- D3560: One-unit ASCII strings from a per-VM cache that every collection drops
- D3550: A primitive's builtin method from the cache slot; the first write to a builtin changes the epoch
- D3551: A `@fast` builtin's call without leaving the loop; the fused call measured and dropped
- D3570: for-of over strings without a trip through `step`
- D3580: A trace frame is named by the function's own string `name`; `$djs.k` copies the origin's
- D3590: Builtins charge fuel for their work: a work counter in the heap
- D3591: The size budget gets 1 KB more for the fuel fix (owner)
- D3592: Host errors carry their data; `values` are host data a dump names
- D3593: The JSON boundary: converting functions in the VM, made by the Sandbox
- D3594: `RunExit`: a host function ends the run uncatchably
- D3595: Bounded steps of host-driven runs
- D3597: Codemode bodies import host modules: leading imports, bound as parameters
- D3600: One wasm module, with the dumper; packaging B goes
- D3602: The npm package has one entry point (0.2.0)
- D3610: Errors, Dates, Maps and Sets cross to the host as data
- D3611: Namespaces are frozen, enumerable, with aliases and a missing-member hook
- D3612: A run ends when its body's result settles; open calls are dropped
- D3613: The JSON boundary per function: `jsonBoundary: { except }`
- D3614: `maxFuel` spans a host-driven run's steps and is exact
- D3620: An argument a host function cannot read is a TypeError at the call
- D3621: The globals the sandbox binds are read-only and non-configurable
- D3622: An Error's name and message are coerced as `String()` would
- D3623: Namespace objects have a null prototype
- D3624: The reader takes an Error's own `stack` before its recorded trace
- D3640: No compatibility code (D024, owner 2026-11-23); what 0297 removed
- D3641: A VM image loads only into the build that made it; the dependency descriptor goes
- D3642: The slim build goes; djs has one feature set of builtins
- D3644: The builtin registry is a current table, not an append-only one
- D3645: djs-embed has no version promise of its own
- D3701: One slot table for the object classes
- D3702: Snapshot bytes are trusted host input, validated for robustness
- D3703: The snapshot gate leg and its negative control
- D3704: The corpus's waiting functions' bytecode is pinned
- D3705: The size ceiling rises by the snapshot module until the dumper goes
- D3710: Builtin records and state constants have named schemas
- D3711: The bytecode hash names `get_builtin` slots
- D3712: A perturbed build continues the corpus's snapshots
- D3713: The size ceiling rises by the named records
- D3720: A host's run is snapshotted with its host values named, and loads into the host's VM
- D3721: Snapshots of lazily compiled VMs
- D3723: The size ceiling rises by host snapshots
- D3740: Lazy function compilation is removed
- D3741: The test-only fast tier stays the reference of the tier-0 loop
- D3742: ABI 12: `vm_new(limit, flags)`, no `vm_frozen_builtins`
- D3743: Builtin registry entries are paths without ids
- D3744: Every djs-wasm module has snapshots and a 128 KiB stack
- D3745: One driver: run and runAsync are loops over the host-driven run
- D3746: Deterministic re-runs are the fallback for refused snapshots
- D3747: ABI 13: snapshot operations without host handles or hashed scripts
- D3750: TypeScript legacy decorators and emitDecoratorMetadata are removed
- D3751: describeSnapshot and callSite are in the docs module only
- D3752: strip-types is a djs-core feature, off in the wasm modules
- D3753: The unchecked release difftest runs the corpus without the stress variants
- D3754: The CLI, the REPL and the docs runner slice in the same VM
- D3755: docs/cli.md and docs/host.md are folded into the guide
- D3756: One meaning per word: an image is a VM's memory, a snapshot is a run

## Embedding in Rust hosts (Gondolin)

- D3170: djs-embed: the native embedding crate for Rust hosts
- D3171: ES modules in djs-core, behind the `modules` feature
- D3190: ES modules: environments are the module closure's captures; records, namespaces, size
- D3191: TypeScript modules drop imports no value refers to
- D3192: Size of 0241; the size ceilings rise by what it measured
- D3193: ES modules are part of `embed`, not a feature of their own
- D3194: djs-embed collects after the cap's out-of-memory abort
- D3172: TextEncoder and TextDecoder (UTF-8) are builtins
- D3173: No URL or URLSearchParams in djs
- D3174: libm becomes a djs crate (djs-libm)
- D3180: djs-core's `embed` feature; the wire format's uncounted UTF-8 strings
- D3181: Reaching the heap cap can end the run uncatchably
- D3182: `Vm::reset_to`: reload an image into the VM's own arena buffer
- D3183: `Vm` is `Send` in every build
- D3184: djs-embed's API and failure model
- D3215: djs-libm: what was copied, and how the build hash covers it
- D3220: Base images are target independent; cross builds are a manual check
- D3225: djs-embed values in and out without a wire buffer; borrowed plain data in
- D3226: Where a hook call's time goes; what was not done
- D3230: test262 runs the module tests: a fixture loader in the runner
- D3231: `import * as ns; export { ns }` is an indirect export of the namespace
- D3175: One djs-core feature for native embedders: `embed` carries ES modules and the encoding builtins
- D3210: TextEncoder and TextDecoder: a TextCodec class, WebIDL in the builtin, the engine's transcoders
- D3211: Size of 0244; the size ceilings rise by what it measured
- D3235: Synthetic modules and `import()` through the host's loader
- D3236: djs-embed's module API
- D3245: JSON modules: the `type` attribute goes to the loader, the source says what is JSON
- D3246: Top-level await: such a module is an async body, found by a first sync parse
- D3247: djs-embed's API changes are listed in its CHANGES.md
