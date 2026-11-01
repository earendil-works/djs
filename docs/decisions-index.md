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
