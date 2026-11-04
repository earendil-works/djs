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
- D1635: Both backends are tested

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
- D1750: The build hash covers features and libm; canonical f32/f16 NaN
- D1431: The base image is compacted before it is collected
- D1340: The prelude keeps no source text
- D1430: Privileged code keeps no line tables or local names
- D1433: Prelude closures carry no `length`, `name` or `prototype`
- D1495: pi adapter: reuse the VM image taken after the prelude
- D1890: pi adapter: snapshot() and the prelude image collect first

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
- D1150: libm is the math dependency of djs-core
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
- D1633: Fuel per transition; the backend is part of the engine's identity
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
- D1112: `\p{...}` is not supported yet
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
- D1651: TypeScript legacy decorators behind a compile flag, as thunks

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

## The pi adapter

- D1490: pi codemode on djs: an adapter for pi's `startVm` contract, pi's prelude unmodified
- D1492: pi adapter: 10,000 fuel per interrupt poll
- D1493: pi adapter: deterministic `Math.random` matches quickjs-wasi
- D1494: pi adapter: quickjs-wasi's host globals in JS, before the prelude
- D1495: pi adapter: reuse the VM image taken after the prelude
- D1660: pi adapter: the prelude is not polled; the fingerprint hashes the host globals
- D1890: pi adapter: snapshot() and the prelude image collect first

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
- D2260: Snapshots are compact containers: collected, zstd against a reference, pinned to the build
- D2261: Snapshot compression runs in the host: node's zstd at level 3; none without it
- D2262: pi's prelude image is the reference of the adapter's snapshots and is recomputable
- D2263: Sandbox.bytes() collects first
- D2282: Difftest ratchet reasons are an excerpt around the first difference
- D2283: Harvested cases with structured tool results
- D2284: Extra redaction for structured-result harvests
- D2041: An old region that minor collections neither trace nor move
- D2042: Full collections do not promote; only seal points make cells old
- D2043: pi's later cold starts size the arena for the sealed prelude heap
- D2044: No card table and write barrier (old-generation phase 2) for now
- D2251: The base image is ordered by cell kind
- D2252: The wasm module ships the base image coded; hosts decode it once per module
- D2331: The shipped module is built with remapped source paths
- D2332: djs-wasm is no_std on wasm32, with a panic handler that never formats
- D2333: The shipped module drops panic locations (-Zlocation-detail=none, the one unstable flag)
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
- D2066: Session images: the roots cell, a canonical prefix diff and the session region
- D023: Notes superseding parts of D2304 after 0147
- D024: No compatibility with unreleased formats
- D2400: One snapshot container format, session images only
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
- D025: The slim build's feature is `full`
- D026: Image compatibility keyed on what an image depends on
- D2520: Arena access: one u64 compare by default, an optional unchecked `unsafe-heap` build
- D2550: Fuel stays per instruction; charging only at safepoints measured, not adopted
- D2551: Large ASCII host strings are copied straight into a reserved heap string
- D2411: The slim build: the `full` and `unicode` features, cut by a constant
- D2412: The registry lists what the slim build lacks
- D2413: Testing the slim build: its own legs in the gate
- D2444: Code inline in the template, the rest of a user function in FuncInfo
- D2310: A twin stays only where it pays its bytes
- D2265: Images carry a dependency descriptor; loading and containers check it, not the build hash
- D2266: RECORDS_HASH: the id tables images store numbers of, read from the sources
- D2267: Cargo features are classified as image features or image-neutral
- D2268: When to bump COMPAT_VERSION
- D2521: Vec pushes out of line outside the hot loops (`PushOut::push_out`)
- D2445: A closure's length and name come from its template until changed
- D2540: A 128 KiB wasm stack; parser nesting limit 550
- D2541: Instance memory: the arena kept at memory's end, lean start capacity, the coded base outside memory
- D2522: `Vec::with_capacity` out of line (`buf::vec_with_capacity`)
- D2552: reduce by argument count, its accumulator in the dense loop's state
- D2433: Keyed string hashing for the VM's own atoms and Map/Set
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
- D2352: djs-wasm leaves the dumper out by default (+32.4 KB gzip)
- D2353: Dumps are Scripts: format version 1 and the promise that later builds load it
- D2354: A wasm host dumps from the heap bytes in TypeScript; the Rust dumper is the reference
- D2355: A dump is checked by compiling it: every function keeps the bindings it read
- D2560: A waiting session persists as a continuation dump; heap images stay inside a process
- D2561: Dumps are taken only at idle points
- D2562: In-flight builtin state is re-enacted; a few privileged loader primitives
- D2563: The compiler's suspension maps are a pinned contract
- D2564: One dumper, in Rust; the TypeScript dumper is dropped
- D2565: djs builds pi's future prelude shape in its own adapter first
- D2590: Corpus tool calls are idle points: `host.later`
- D2600: The CLI's `tools`: a Proxy that makes one async host function per name
- D2601: Scripted tool answers: by arguments, else in order
- D2570: Suspension maps from a role stack the parser keeps
- D2580: While a dump loads, static initializers skip static fields and blocks
- D2581: Format 2 is a Script run by `Vm::load_dump` with the loader's `$dump`
- D2582: Classes and private fields in dumps
- D2583: The format 2 promise, concretely
- D2584: `$dump.resume` builds the frame from the map of the dump's own text
- D2585: A load's open calls, handles and call ids
- D2602: The CLI's host globals `later` and `print`
- D2603: Resuming dumps in the CLI
- D2610: The exporter writes what is in flight as the calls that make it
- D2611: Variables of one name in several scopes get nested factories
- D2612: Re-enactments of iterators and for-await state
- D2613: The gate dumps at every idle point of the corpus
- D2614: The dumper's size in the dump module
- D2615: A dump holding TypeScript functions is TypeScript
- D2616: Where activations stopped is reported, not written as comments
- D2640: The dump format marker is the directive `"djs-dump 2";`
- D2641: Dumps carry no comments; `djs resume --explain` explains them
- D2617: A codemode body is dumped as the async function it runs as
- D2618: A script binding with a host name is the script's
- D2604: Dump commands in the CLI: idle points numbered by answers
- D2619: Golden format 2 dumps change only with a decision
- D2630: Host-driven Sandbox runs, the first piece of the Sandbox run API
- D2631: The dump exports carry the host's function names both ways; changed in place
- D2632: Dotted host function names make a namespace object
- D2620: A codemode body's frame is an async function's
- D2633: A dump names the run's result without a handle; `vm_outcome` removed
- D2634: `run`, `runAsync` and `replay` are built on the host-driven run
- D2635: Both dumper packagings, chosen by the host; the image moves between modules
- D2621: A class's hidden bindings in a waiting frame are dead; private names are the class's
- D2622: Format 2 dumps carry no helper code; the helpers are loader primitives
- D2623: Computed keys the dump would evaluate again are refused
- D2624: A dump carries Math.random's state
- D2625: The golden dumps are rewritten for D2622 and D2624
- D2650: pi's future shape in the adapter: prelude-next, `next` mode, session dumps
- D2651: `dumpEveryIdle`: the next mode's stress mode
- D2652: prelude-next has no lockdown; the next mode has frozen builtins
- D2653: The pi adapter counts fuel across sync host calls
- D2700: Heap images are no longer a persistence format; what was removed
- D2701: The dump module's instance and VM are kept between dumps
- D2702: The Sandbox's `maxFuel` counts fuel across sync host calls
- D2703: Hosts store dump text compressed with zstd; the format stays a plain Script
- D2704: Packaging B: the shipped module has no dumper, hosts that persist load the dump module
- D2201: The guide's site follows Elanor d3184572: its retuned type scale
- D2626: A dump's builtin names are walked once per VM, and walked faster
- D2627: Subroutine return addresses in dumps are gosub indexes; case tests keep their discriminant
- D2628: Dumps write using Records, destructuring, for-in and builtin iterators from their state
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
- D2770: A call's state is JSON text, written verbatim as `$dump.pending`'s fourth argument
- D2771: Tasks: the Sandbox drives the run and the open calls' hooks
- D2731: Slots name every frame index; the operand stack stays in memory (refines D2720)
- D2772: The pi adapter's open calls carry their pi ids as call states
- D2790: No doctests, no empty test binaries, no bgen program in the gate; guarded
- D2791: One integration-test binary per package
- D2792: Builds end when cargo does; first runs overlap the legs
- D2732: Construct temporaries: statement state in named locals, listed where live
- D2733: Finally completions in temporaries; gosub/ret and return-address indexes gone
- D2734: No role vocabulary: stack values are named by construct; class sites refused by a marker
- D2735: One compile per load: the loader records the maps in the compile that runs the dump
- D2800: String and number constants are not deduplicated per function while parsing
- D2801: With the dumper, pattern scans stop at a literal element
- D2802: One dump-module sandbox per module and memory limit in the process
- D2830: Continuation dumps become plain JavaScript
- D2831: The plain dump is written by the Rust exporter
- D2832: Open host calls and Math.random's state are restored through host functions
- D2833: Re-dumps use origin tracking
- D2850: Suspension maps carry the syntax around each site
- D2890: Plain dumps load as a Script with `$djs.resume`/`$djs.seed`; open calls keep their ids
- D2891: Hosts load plain dumps themselves; sync host functions run meanwhile
- D2892: The size ceilings rise for plain-dump loading in the shipped module
- D2851: Site syntax: member targets split, recording only in async functions and generators, pinned
- D2893: pi's next mode names its host functions `$djs.bridge`/`$djs.call` in plain dumps
- D2894: The CLI's open calls carry their origin and the answers file's counts as state
- D2852: Destructuring patterns are site-syntax nodes
- D2853: TypeScript expressions are transparent on site paths
- D2895: Several handles in a plain dump: a null-prototype object as the completion value
- D2896: The gate clears a corrupt incremental cache and runs the job once more
- D2860: The plain exporter writes format 2's lines and an IR, and rewrites them
- D2861: Plain dumps are checked over every corpus idle point, in djs and in node
- D2870: Origin tracking reads a continuation's frame as its origin's
- D2897: `$djs.seed(lo, hi, next)`: call ids go on after a plain load as in the uninterrupted run
- D2898: The Sandbox writes plain dumps by default
- D2871: Plain continuations of methods that use `super` are methods of the home's prototype
- D2872: `for await` in plain continuations: a loop over the saved record, the iteration in flight before it
- D2873: Origin tracking inside a continuation's rewritten statement, case tests, finally by jumps
- D2874: Destructuring around a continuation's site is written as a pattern again
- D2834: Plain dumps are mostly self-contained: the code to continue is in the dump, the host provides `$djs`
- D2835: The dump's first line is `"djs-dump";`
- D2900: A plain dump's `$djs` record leaves default fields out
- D2901: The plain dump size score
- D2902: Anonymous closures in a scope factory are written as `key: (0, f)`
- D2875: `yield*` in plain continuations: a `yield*` over a wrapper of the inner record
- D2836: Plain dumps are written only for VMs with frozen builtins
- D2903: The plain dump runtime: the format's helpers are members of `$djs`
- D2876: `using` in plain continuations: the resources added again, a disposal as a scope of K
- D2877: Iterator helpers in plain dumps are made again by their public methods
- D2878: Plain dumps refuse instances that carry private names
- D2904: The hosts' plain-dump paths run with frozen builtins
- D2879: TypeScript plain dumps are written as JavaScript where the TypeScript is only types
- D2880: Sites in loop heads, `var` declarations, catch parameters and more literals
- D2881: Classes in a scope a waiting frame shares go into the continuation's block
- D2883: Classes that name themselves dump plainly; a class's own static private fields
- D2960: Exception handlers in unreachable code are dropped
