# Board

Owned by the coordinator. Status of all issues; details are in `issues/`.

## In progress

| issue | title | agent | worktree |
| --- | --- | --- | --- |
| 0201 | Plain-JS exporter in Rust (step 2), then 0202, 0203 | pjx | .worktrees/pjx |
| 0204 | Host hooks for plain dumps (step 5) | pjh | .worktrees/pjh |

## Open (ready)

| issue | title | depends-on | touches |
| --- | --- | --- | --- |

## Open (blocked on dependencies)

| issue | title | depends-on |
| --- | --- | --- |
| 0202 | Origin tracking (step 3) | 0201 (agent pjx) |
| 0203 | Plain encodings for remaining refusals (step 4) | 0202 (agent pjx) |
| 0205 | Retire format 2 (step 6) | 0203, 0204 |
| - | Gate: rustc incremental corruption in .worktrees/gate (linker Undefined _anon.*.llvm.*): auto-clear incremental dirs and retry once, or CARGO_INCREMENTAL=0 for gate builds (measure) | - |
| - | One djs-prelude build script per target dir (~20 fresh executables per core change): fewer cargo configs or a base image without that build script | owner (design) |
| - | djs-wasm base_equals_a_compiled_realm fails (--features compiled-prelude, no gate leg) | - |
| - | CLI/REPL dumps write no host-call states | - |
| - | macOS Developer Tools for the agent host app (first-exec notarization checks, 30-80 s per new binary) | owner (GUI) |
| - | Blob buffers in dumps; persist() convention for pi (0167 follow-ups; the TS dumper is dropped, D2564; classes are 0170) | owner |
| - | Vec push_out/with_capacity in bloat's files (~1 KB); scratch buffers for Vec drops (few KB) | 0151 L3 |
| - | Result buffers + snapshots referencing tool results (0165 steps 2-3) | dropped with image persistence (D2560); revisit for dumps |

## Done

| issue | title | merged |
| --- | --- | --- |
| 0001 | Workspace skeleton and process | initial commit |
| 0003 | Lexer | e4a42c3 (agent lexer; parser follow-ups listed in the issue) |
| 0005 | Values, arena heap, strings, atoms, objects | dc751e2 (agent heap; follow-ups in the issue) |
| 0023 | djs-wasm + JS host package | d4fb4a4 (agent num; djs runs in node; vm_new 11 us; 132 KB gz at opt s) |
| 0028 | P1-2 runtime | 468f874 (agent heap; prelude at Vm::new: 92 us native / 146 us wasm; Error stack) |
| 0029 | P1-5 Loops and iteration (compiler + iter builtins) | 8cc5aa2 (agent lexer; +1,774 test262) |
| 0030 | P1-5 runtime + 0028 leftovers + GC audit | 9f35b6f (agent heap; 4 parts) |
| 0041 | Async/await, generators (compiler) | f93fa30 (agent lexer; codemode body is a real async function) |
| 0043 | wasm size audit (pass 1) | a35c5e8 (agent heap; stripped 1,200 -> 948 KB; base 502 -> 288 KB; second code pass deferred until wasm-opt numbers) |
| 0042 | Async generator runtime, function kinds, host rejections | 6e3c965 (agent heap) |
| 0037 | P2 Promises, jobs, async host calls, suspendable frames | bd6f974 (agent heap; 3 parts) |
| 0036 | P1-6 Destructuring + tagged templates | 234fd3c (agent lexer; +~1,940 test262) |
| 0052 | Bloat investigation | 4fe04f1 (agent bloat; report + 5 analysis scripts) |
| 0057 | Size quick wins Q1-Q6 | f26f265 (agent bloat; code -18%; main now 1,241 KB, 428 KB gz) |
| 0048 | P3 TypeScript | c3261f9 (agent lexer; strip-types 4,053/4,053 byte-identical vs ts-blank-space) |
| 0050 | RegExp completion | e214e54 (agent num; \p{} on by default +27 KB gz; RegExp promoted; pi image regex fixed) |
| 0046 | BigInt | 63d8468 (agent heap; BigInt 1135/1142; active 98.8%) |
| 0033 | JSON + RegExp builtins, regex literals | 8b5ccfb (agent num; active 32,303 / 32,718) |
| 0040 | P1-7 Classes | 2d08a6c (agent lexer; test262 active 97.1%) |
| 0096 | Compiler structural speedups | 5547d5ec (agent lexer, archived; compiles -8..-10% more; typescript.js now fewer instr than quickjs-wasi) |
| 0092 | Compiler size + compile speed | 79a2faa8 (agent lexer; typescript.js compile -37% wasm instr; compiler -7% size) |
| 0086 | Bytecode quality | 8fbd04f (agent lexer; tight -38%, prop1 -33%, call1 -22% wasm instr; renumbering measured slower, not merged) |
| 0084 | Long tail part 2 | a7a38ed (agent lexer; test262 failures 56 -> 43, all by design or routed) |
| 0082 | The long tail | 5a4ba6a (agent lexer; test262 failures 203 -> 56; Known differences in TESTING.md) |
| 0065 | Decorators (TC39 + accessor + TS legacy) | f4894f5 (agent lexer; strip-types 4,316/4,802, 0 differ) |
| 0058 | Compiler: using, completion values + Goal::Eval, import() | 8bc39dd (agent lexer; strip-types 4,085/4,802, 0 differ) |
| 0075 | Build identity + Float32 NaN | df34400 (agent num; D1750; libm pinned =0.2.16) |
| 0076 | Unicode normalization | 9825c90 (agent num; +20.5 KB gz, feature on by default D1760; 400,680/400,680 conformance) |
| 0061 | String append + JSON source text | dbd2e8d (agent num; 10M appends 574 -> 150 MB RSS; JSON 142/142) |
| 0059 | Indirect eval, Function ctors, queueMicrotask | 4435b3b (agent infra; active 37,464 / 37,722 = 99.3%; pi 215/226) |
| 0064 | djs repl | 31797b6 (agent repl, archived; docs/cli.md; follow-ups: tab completion, interactive .step) |
| 0077 | deser deps investigation | report (agent deps, archived): no new deps; quick wins -> 0078 |
| 0089 | pi snapshot collect fix | 68343b1 (agent snapshot, archived; snapshots 936 -> 498 KB; DETERMINISTIC_VERSION 2) |
| 0088 | Snapshot design research | 6c0e381 (agent snapshot; recommends A now, B long-term, defer C; owner decision pending) |
| 0110 | pi-like library hot spots | 51a775b8 (agent perf; pi-like -14.3% wasm instr; D2090) |
| 0107 | Direct native-to-closure calls | 86e9ef07 (agent perf; higher-order -13%, forEach -12.8% wasm; D2060) |
| 0100 | Native codegen: typed locals in Rust | 740803ce (agent perf; forEach -34%/elem wasm, higher-order -10%) |
| 0093 | run_hot: tier-0 loop + compact calls | 22495ce3 (agent perf; plain bytecode 0.44-0.63x quickjs-wasi; calls ~1x) |
| 0085 | @fast leaf entries (57) | 0dd2c96 (agent perf; pi-like 3.6x -> 1.8x quickjs-wasi in wasm instructions) |
| 0081 | Perf anatomy | report (agent perf): plain bytecode at par in wasm; gap is builtins/iteration; P1-P10 |
| 0072 | Determinism checks in the gate (both parts) | 38f0c91 (agent determinism, archived; D1720, D1725) |
| 0104 | wasm size pass 2 | c154d535 (agent num, archived; -29.4 KB raw, -4.9 KB gz; follow-ups in owned areas) |
| 0099 | Cold-start memory + verify bitmap | 27314d59 (agent num; cold return-1 3.81 -> 2.81 MiB; tool-heavy cold 4.00) |
| 0094 | Baseline wasm memory | ade72f0f (agent num; pi warm start 3.50 -> 2.13 MiB; stack 1 MiB -> 256 KiB) |
| 0087 | Allocation churn + wasm memory | 2d24853f (agent num; strbuild 57.8 -> 35.5 MiB, json 21 -> 14; to-space reuse measured worse, dropped) |
| 0078 | Leaf quick wins | 73836fe (agent num; wasm ASCII 4x, non-ASCII wire decode -35..40%) + test262 property-escape speedup 1cbfc29 (D1783-D1785; gate test262 187 -> 133 CPU-s) |
| 0071 | GC points + raw images slice-independent | f14da99 (agent num; D1710, D1711) |
| 0074 | Fuel overshoot carried across runs | 793e29d (agent determinism; D1740) |
| 0073 | Replay journal + canonical hash | 37e6172 (agent determinism; D1730) |
| 0068 | Deterministic mode for VM and Sandbox | 971036e (agent determinism; D1680) |
| 0067 | Host inbox (async host results as macrotasks) | 7988b76 (agent determinism; D1670) |
| 0066 | pi adapter determinism fixes | 6c523da (agent determinism; equal polls cold/warm; pi suite 221/226) |
| 0055 | Determinism investigation | b30f154 (agent determinism; native/wasm images already identical; issues 0066-0075) |
| 0079 | Integration efficiency | 6ec6e37 (agent devflow, archived; lock 89% -> 47%; merge-stats.sh; slow-test report) |
| 0053 | Dev-flow test fixes | e1f4d63 (agent devflow) |
| 0190-0196 | SM migration: slot names (D2730), named slots (D2731), construct temporaries + iter_next_loc (D2732), finally completions (D2733), roles deleted (D2734), one compile per load (D2735) | 9cc8741b (agent sm) |
| 0198 | Dump performance: corpus dump p50 1.42 -> 0.43 ms wasm, totals 3,243 -> 610 ms; data cases 2-6x (D2800-D2802) | 429184db (agent dumpperf) |
| 0206 | Site records: pattern nodes (D2852), transparent TS (D2853), Decl.at, ctor params | 10eac128 (agent pjc; 1,063/1,063 complete paths) |
| 0200 | Compiler site-structure records for plain dumps (D2850, D2851) | 0cd63ec1 (agent pjc; 1,062/1,063 sites with complete paths) |
| 0199 | Plain-JS dumps prototype: continuations with origin tracking; 1,161/1,161 harvested identical in djs and node; bounded re-dumps; recommends switching (D2830-D2833 proposed) | cdd46c56 (agent plainjs) |
| 0197 | Gate waits: bgen and doctests legs dropped, one test binary per package, targets guard (D2790-D2792) | 823103e5 (agent gate-speed; core change gate ~65-89 s) |
| 0189 | Host calls as data: per-call state, {call, save, resume}, tasks; sleep example 77 lines (D2770, D2771) | 891fc12c (agent hostcalls) |
| 0195 | pi adapter next mode: call ids as call states (D2772) | 6a8759bd (agent hostcalls) |
| 0188 | Suspension design review: switch to SM (state machines in the compiler); residualizer prototype | d7b62185 (agent critic) |
| 0183 | Gate in seconds: gate.py legs + input-hash cache, merge lock only for ff, clippy combos, --full worktree (D2660-D2669) | 1638b342 (agent gate-speed; docs 249->8 s, js 252->12 s, core 407->127 s) |
| 0187 | Removed djs heap and .image; .heap kept (D2269) | 6d857a55 (agent snapshot) |
| 0184 | Dump refusals closed (finally/switch D2627; using/destructuring/for-in/matchAll/helpers D2628); cached name table D2626; dump costs measured | fbb35ee1 (agent portable; 0 refusals at 1,063 idle points) |
| 0186 | ARCH/TESTING: dumps persist, images in-process, packaging B | e6705afe (agent example) |
| 0185 | Guide on Elanor d3184572: smaller type scale, inline code scale, tokens license (D2201) | e18ca671 (agent design) |
| 0177 | Images in-process only; packaging B; dumps stored zstd; maxFuel fix (D2700-D2704) | 62413af5 (agent example; shipped 440.5 KB gz) |
| 0176 | pi's future shape: prelude-next, adapter next mode, dumpSession/resumeFromDump, fuel across sync calls (D2650-D2653) | 380df83a (agent example) |
| 0182 | No helper code in dumps (D2622), computed class keys refused (D2623), Math.random in dumps (D2624), goldens D2625 | 8a9a7e70 (agent portable; sleep dump 3,159 -> 1,308 B) |
| 0178 | Dumps across builds: golden fixtures, two-commit check (D2619); exporter fixes D2617-D2621 | 4ee8c2c2 (agent portable) |
| 0175 | Sandbox run API on host-driven runs (D2633-D2635); packaging B (dump module) | 7f380233 (agent example; dumper +96.8 KB gz) |
| 0174 | REPL/CLI: tools, waiting inputs, .dump/.restore/.dump-every, run --dump-after, resume (D2600-D2604) | 242f8294 (agent repl) |
| 0179 | examples/sleep-dump: dump while a host tool sleeps, restart, continue; Sandbox begin/resume (D2630-D2632) | 19fe4b71 (agent example) |
| 0180 | Dumps without comments: "djs-dump 2"; directive (D2640), djs resume --explain (D2641) | dd98ec06 (agent nocomment) |
| 0181 | atob/btoa/performance.now: already provided (engine + pi adapter); docs/web-globals.md | b73b16c0 (agent globals) |
| 0172 | Exporter: continuations, re-enactment, dump_report; --dump-every-idle in the gate (D2610-D2616) | 23835fa9 (agent portable; 1,197 idle points identical) |
| 0171 | Loader: $dump.resume, cells, pending calls, handles; v2 fixtures (D2583-D2585) | 38a37048 (agent portable) |
| 0170 | Classes and private fields in dumps; Vm::load_dump with $dump; jump_if_loading (D2580-D2582) | 8b78bca5 (agent portable) |
| 0169 | Suspension maps in the compiler, pinned (D2570) | e43e7e20 (agent bloat; 1,063 sites in 315 cases, 53 unsupported) |
| 0173 | Corpus idle points via host.later (D2590) | 90aa5acf (agent infra; 1,194 idle points in 294 cases) |
| 0168 | Dump bug: closures held directly in bindings skipped the factory; compile-based checker (D2355) | 494dbd00 (agent portable) |
| 0167 | Semantic dumps: Vm::dump, .dump/.restore, Sandbox.dump/load, TS dumper prototype (D2352-D2354) | 6be9cd54 (agent portable; off in djs-wasm: +32 KB gz) |
| 0151 | Base heap 325 -> 186 KB, every builtin kept (D2440-D2450) | 88356782 (agent bloat; -42.7%, 3,100 cells) |
| 0162 | pico speed findings: host strings in place (D2551), reduce one frame (D2552), species/ArrayCreate (D2554), stringify into heap (D2553); fuel rule not adopted (D2550) | 0c31e693 (agents speed, speed2) |
| 0166 | Parser depth kept 550, regexp 212 (D2545); GC thresholds from BASE_END (D2544); stack overflow reported (D2543) | 866e73fd (agent heap) |
| 0165 | Sliced strings (D2542); steps 2-3 deferred | 01817649 (agent heap; log-5mb arena 12.0 -> 7.2 MiB, snapshot 1.75 -> 0.98 MiB zstd) |
| 0163 | Seeded hashing: SipHash-1-3, per-VM key (D2433) | 4b3bff97 (agent strings; COMPAT_VERSION 2) |
| 0161 | Safe heap access (D2520), unsafe-heap feature, Vec hygiene (D2521, D2522) | c169e192 (agent audit; -9.7 KB gz; module 441.8 KB) |
| 0158 | Instance memory: fresh Sandbox 18 -> 11 pages (D2540, D2541) | 47d32099 (agent heap; 128 KiB stack, MAX_DEPTH 550, coded base outside memory, ABI 2) |
| 0164 | Image compatibility by dependency descriptor (D2265-D2268) | cb43e070 (agent snapshot; container format 3) |
| 0159 | Twin re-benchmark: 54 @fast + 2 @native dropped (D2310); wmeasure fails loudly | ec8042f5 (agent perf; full 445.1 KB, slim 379.3 KB) |
| 0160 | Slim build: full + unicode features (D2411-D2413, D025) | 7074117e (agent slim; full 450.4 KB 1.57x, slim 385.8 KB 1.35x; gate legs) |
| 0155 | instr.py fixed + user/builtin split (D2309); no stale stack slots in images (D2308) | f1095c99 (agent perf; pi-like: 47% user bytecode, 39% builtins, 12% GC) |
| 0156 | Architecture bloat audit (research) | 4f53e325 (agent audit; docs/architecture-bloat-audit.md: no big misstep; floor ~0.75-0.8x; memory on par with qjs) |
| 0157 | Pages deploy retry | b7233a0f (agent pages) |
| 0153 | Wasm JIT study (D2510: not now) | b6fd22ff (agent jit; docs/wasm-jit.md; prototype 3-8x after TurboFan, but scripts too short) |
| 0154 | Slow spots from pico benchmarks (D2305-D2307) | 0917b848 (agent perf; keys spread 593 -> 44 ms; Array.from length 168 -> 101 ms; tiny callbacks -10..-40%) |
| 0152 | Lessons from pico (research) | 60fafe0a (agent pico; docs/lessons-from-pico.md) |
| 0150 | Base string dedup, two-level atom table, analyzer session view (D2430-D2432) | eb57178a (agent strings; fresh REPL session image 34.6 -> 11.5 KB raw) |
| 0149 | Slim codemode build (research) | 9d3a9c8c (agent slim; docs/slim-codemode.md: slim cargo feature + second module, 399.6 KB gz prototype, 1.39x) |
| 0148 | Single snapshot format (D2400); builtin-created shapes pre-created (D2067) | de5efbcf (agent heap; builtins add nothing to the session region in any order; +648 B, props +1.4%) |
| 0147 | Immutable builtin prefix, session images (D2062-D2066) | ae8771c7 (agent heap; prefix write barrier; DJSN format 2; empty Sandbox 1 KB; builtin-created shapes still order-dependent -> 0148) |
| 0146 | Frozen-builtins VM option (D2304) | af8497bb (agent perf; flag on <=0.8%; pi start w/o lockdown 3 ms -> 0.40 ms, 480 -> 376 KB) |
| 0145 | Stable append-only builtin registry (D2371) | 09dfb743 (agent bloat; 1,409 entries; gate check; module size unchanged) |
| 0144 | Frozen builtins: design + prototype (research) | e919a0a1 (agent heap; docs/frozen-builtins.md: immutable frozen prefix, no laziness; separate space +3-6%) |
| 0142 | Goldilocks snapshots (research) | e979b0ee (agent portable; docs/goldilocks-snapshots.md: layered reference + two tiers) |
| 0143 | Why a fresh heap is 300 KB (research) | 89a615a3 (agent bloat; docs/empty-heap.md: lazy builtins -> ~10 KB fresh VM, 8-11 weeks) |
| 0141 | Portable snapshots: design + cross-build prototype (D2350) | ff288fa1 (agent portable; docs/portable-snapshots.md; 1,057/1,057 corpus idle points transfer across 406 commits) |
| 0140 | Snapshots across djs versions (research) | 8b50738b (agent snapshot; docs/snapshot-versioning.md: journal as durable record, snapshot as same-build fast path) |
| 0137 | Callback loops via %EachDense (D2303) | 9dddc84c (agent perf; pi-like -7.7% -> 0.87x quickjs-wasi, higher-order -31%, -196 B) |
| 0139 | Base image becomes the first VM's arena (D2060) | 91a406d0 (agent heap; -0.31 MiB per Sandbox/pi instance) |
| 0138 | Shared read-only builtin segment: design study (not built) | 5d66ef74 (agent heap; docs/shared-segment.md: don't build now; lazy variant 6-7 weeks) |
| 0136 | Base layout stability: growable cells last, atom growth 1.25x (D2057, D2058) | c4993afa (agent heap; empty Sandbox snapshot 15.7 -> 0.8 KB, REPL dump 16 -> 0.8 KB) |
| 0129 | pi-like hot spots: JSON word scans, slice/join, CreateDataProperty (D2300-D2302) | 7c631680 (agent perf; pi-like -12.7%, 0.94x quickjs-wasi) |
| 0133 | Heap dump analyzer, Rust container (D2264) | 104f8a1e (agent snapshot; djs heap; snapshot sizes re-measured) |
| 0135 | Why heap dumps are big: REPL containers, Heap tab own-vs-base (D2340) | aee70c1b (agent bloat; REPL dump 341 -> 16.4 KB; 3,723 of 5,768 base cells are builtin functions) |
| 0134 | Faster old-region walk (D2055); young-sized threshold rejected (D2056) | e4257900 (agent heap; pi-like -3.3%, props -3.3%) |
| 0132 | json peak regression analysed, accepted (D2054) | 0616aeab (agent heap; + pi-like GC profile: 9.1% of instructions) |
| 0131 | Size: decomposition stream, derived compositions, message builder (D2337-D2339) | ef739b01 (agent bloat; -6.3 KB, 454.6 KB, 1.586x) |
| 0130 | Codemode feature usage (research) | b0d92946 (agent infra; docs/codemode-feature-usage.md; no Unicode props/normalization -> 420 KB, 1.47x) |
| 0128 | Page allocator in djs-wasm (D2053) | 0163f92b (agent heap; peak memory halved on big heaps: strbuild 38.6 -> 19.5 MiB) |
| 0126 | Size: paths, no_std panic handler, location-detail, Unicode recoding (D2331-D2336) | acf388af (agent bloat; ~-20 KB gzip; main 460.9 KB, 1.608x) |
| 0127 | Size: internal names out of image, natives pruned, inline threshold (D2253-D2256) | f39803d1 (agent perf; 472 -> 462 KB with 0126 Unicode, 1.612x) |
| 0125 | GC: full first collection; aging rejected; perturbation method (D2045-D2048) | c38968ff (agent heap; strbuild peak back to 25.5 MiB) |
| 0122 | wasm size: image ordering + codec, size gate (D2251, D2252) | 560008b9 (agent perf; 529.9 -> 509.5 KB gzip, 1.78x) |
| 0123 | Why djs is bigger than QuickJS (research) | 36fe5b14 (agent bloat; docs/size-vs-quickjs.md: ~half Rust density, ~half the bytecode image) |
| 0120 | Old generation phase 1 (D2041-D2044) | 4f407762 (agent heap; GC time -8..-51%, pi-like -5.7% instr, pi cold peak 3.00 -> 2.25 MiB; phase 2 not built) |
| 0124 | Every file reference in the guide links to GitHub (D2104) | c432fcbf (agent docs; 117/117 guide path mentions linked; check enforces) |
| 0121 | Corpus expansion to 300 cases; short ratchet reasons (D2282-D2284) | e06e0d20 (agent infra; 295/300 vs qjs; follow-ups: fake tool discovery +404 runs, generic results +429) |
| 0117 | Compact snapshot images (D019, D2260-D2263) | ff1048b1 (agent snapshot; waiting pi session 546 KB -> 2-9 KB) |
| 0119 | Live editor for guide examples (D2103) | 541b30cb (agent docs; textarea overlay + JS port of highlight.rs, tested for parity) |
| 0116 | JSON in place, flat_fast, search (D2250) | 5c73183d (agent perf; pi-like -4.3%, json-indent -13%) |
| 0118 | Redacted codemode corpus (150 cases) | e08736a7 (agent infra; djs matches qjs 145/150, 5 = qjs bug) |
| 0115 | Guide self-contained (owner rule) | f829f68d (agent docs; D2102; goals.md; enforced by check) |
| 0114 | Guide design: Elanor | f93c1058 (agent design, archived; vendored published packages, D2200) |
| 0113 | Docs follow-ups | 6f687be1 (agent docs; generated status D2101, search, live disasm) |
| 0103 | Human-facing guide (djs-docs + 10 chapters) | 46667f42 (agent docs; Pages workflow) |
| 0112 | Guide chapter: determinism | d8282419 (agent infra, archived; guide complete) |
| 0111 | Prototype property order | d5e74b9a (agent infra; D2150) |
| 0109 | Guide chapters: builtins, host, testing | 667f8efa (agent infra; reviewed by docs) |
| 0108 | Name inference, syntax messages, statics order | 392ec528 (agent infra; D2070-D2072) |
| 0106 | Stack follow-ups | e9887a60 (agent infra; prepareStackTrace D2050; <input> frames D2051; prototype-last order D2052) |
| 0105 | Stack trace details | 5c512a7d (agent infra; stackTraceLimit; qjs call positions; difftest stacks directive) |
| 0102 | pi polish | 1d5336a7 (agent infra; pi suite 221/226 unchanged; captureStackTrace; REPL completion and .step) |
| 0097 | Finish the later group | 443e5933 (agent infra; later group empty; active 38,069 / 38,109 = 99.9%) |
| 0095 | Docs coherence + README | 7a4780bb (agent infra; README, ARCH reviewed, decisions-index) |
| 0091 | dense leaves -> @fast, fromCharCode/URI | c62bcc01 (agent infra; arrays/strings now ahead of quickjs-wasi in wasm instr; D1910) |
| 0090 | Natives retry, resizable TA, JSON slow path | dee4dc9b (agent infra; JSON with a Date 18.1G -> 1.2G wasm instr) |
| 0080 | for-of/iteration fast paths + natives wave 2 | fd5a6d1 (agent infra; for-of 5,433 -> 738 instr/elem) |
| 0063 | D015 + bgen Rust backend v1 (35 natives) | 9f662b7 (agent infra; higher-order 6.8x -> 2.3x qjs; +80 KB code) |
| 0101 | Mark-compact collector (D018, D2040) | 1a41ac7b (agent heap; peak memory -3..-52%; Cheney kept for canonical_hash) |
| 0098 | Base image + shapes code size | 87c5e0ce (agent heap; base.img -21.5 KB; module -25.6 KB; atom 3/4 load rejected) |
| 0083 | Shapes (parts 1-3) | 9e7bcfd5 (agent heap; D1831-D1832; objects -27..-33% bytes; literals/ctors ~3x faster) |
| 0069 | Host OOM status | 893495f (agent heap; D1690) |
| 0070 | Collect-and-retry at a failing allocation | 42b7fc5 (agent heap; D1700; natives not yet) |
| 0062 | Property access ICs (part 1) | 5671e8c (agent heap; global read 10.4 -> 1.3 ns; props -14..-18%) |
| 0060 | Out-of-memory recovery | b7c83a8 (agent heap; D1600 reserve, D1601 GC threshold in image) |
| 0056 | String/regex hot paths | 6d24f26 (agent num; test262 active run 250 -> 90 CPU-s, 61 -> 17 s wall) |
| 0051 | Remaining builtins | df48233 (agent heap; all promoted features 100%) |
| 0054 | pi follow-ups | 69ee33c (agent infra; pi suite 213/226; OOM recovery bug -> heap) |
| 0049 | pi codemode adapter | e39a47e (agent infra; pi's suite 204/226 on djs; start 0.49 ms vs 3.2 ms) |
| 0047 | Dev-flow investigation | report in issue file (D012 adopted) |
| 0044 | djs runs the shared difftest prelude | a214890 (agent infra; difftest ratchet 52 -> 38) |
| 0045 | Proxy | 7427f4f (agent infra; Proxy 352/375; ~5,000 behavior cases vs qjs) |
| 0039 | Binary data | c8aefb6 (agent infra; DataView 445/449, TypedArray 841/849) |
| 0038 | Builtins wave 3 | e43d075 (agent infra; Map/Set/Weak*/Reflect/Date; test262 active 64.8%) |
| 0034 | Core builtins wave 2 | a05575b (agent infra; Array 2,461/2,680, String 688/1,040; gate back to 60 s) |
| 0035 | Interpreter call-path performance | 2779e53 (agent heap; calls 7.5x -> 3.35x qjs, loop 9.6x -> 3.8x) |
| 0031 | Core builtins wave 1 | 25fdfa7 (agent infra; test262 active 2,961 -> 7,687; leftovers to 0034) |
| 0032 | Base snapshot | 60a816e (agent num; vm_new 144 us -> 0.4 us in wasm) |
| 0026 | Leaf performance sweep | 04fc8fa (agent num; stringify -24..-54%, GC -11..-19%; opt-level stays s) |
| 0007 | test262 on djs + ratchet in gate | 2312924 (agent infra; 2,961/20,124 active; gate runs all active in 4.2 s) |
| 0016 | djs in difftest + gate | b01a03a (agent infra; 50/154 pass; djs 4-9x slower than qjs on calls/loops/props) |
| 0027 | P1-2 Objects/privileged mode (compiler) | 2e11de2 (agent lexer; test262 compiling 26.0k) |
| 0022 | E2E wiring | f235cdd (agent heap; djs run works; 29 e2e cases x 3 modes) |
| 0025 | P1-3 Exceptions + P1-4 leftovers | d45e837 (agent lexer; test262 compiling 24.7k) |
| 0024 | P1-1 Functions and closures (compiler) | ad6c70f (agent lexer; test262 compiling 11.4k -> 23.5k) |
| 0014 | P0 compiler | 5c5ce95 (agent lexer; 31 e2e programs match qjs; test262 sweep no panics) |
| 0021 | Outbound direct heap reader | 91b5ae5 (agent num; 1 MiB string at memcpy speed; 1.9-5.9x faster than JSON path) |
| 0015 | P0 interpreter and embedding | 2373382 (agent heap; P0 + most builtin-free P1 ops; e2e left to 0022) |
| 0020 | Flaky difftest timeout | 6818440 (agent infra; D1200 timeout convention) |
| 0018 | Inbound wire format | 95afebe (agent num; beats JSON path on all datasets; node tests in gate) |
| 0019 | Benchmark harness | 0bb9d9b (agent infra; baseline in tests/scores/bench.tsv; wasm 181 KB stripped, 85 KB gz) |
| 0017 | djs-bgen v0 | 8496b74 (agent infra; Rust, no deps; D1170-D1173; --check in gate) |
| 0011 | Regex engine core | d57c4a9 (agent num; 13.6k qjs cases; \p{} deferred, D1112) |
| 0013 | Bytecode table and tools | 96c4a0a (agent lexer; 147 opcodes) |
| 0012 | Native JSON (leaf) | 9f437fa (agent heap; parse ~80% of node, stringify ~50%) |
| 0010 | Differential corpus and difftest runner | c5039b3 (agent infra; 165 cases, all pass on qjs) |
| 0008 | Compiler and bytecode design note | 977acdd (agent lexer; docs/compiler.md; D006 accepts it) |
| 0006 | P0 umbrella | split into 0013-0016 |
| 0009 | Copying GC and handle table | 6030aff (agent heap; ~15 ns/cell; LOS designed, not built) |
| 0004 | Number <-> string conversion | 9b0a654 (agent num; matches qjs on 2.1M cases; D1041 differs on >38-digit inputs) |
| 0002 | test262 infrastructure and oracle baseline | 5087390 (agent infra; qjs: 37,569 tests in ~10s, 4 oracle failures) |

## Backlog (not yet written up)

- P1 (split in docs/compiler.md section 10, P1-1..P1-10): functions + closures + the resolution pass, objects/arrays literals, exceptions,
  let/const/TDZ, template literals, destructuring, spread, arrows, classes. The Cheney GC.
- P2: promises, async/await, job queue, async host calls, djs-bgen (checker + JS emitter),
  builtins in the builtin language, base snapshot, JSON, core builtins, djs-wasm, JS host
  package.
- P3: TypeScript type skipper (+ the `strip-types` oracle vs ts-blank-space), enums,
  namespaces, parameter properties.
- P4: snapshot/restore API, deltas, old generation.
- P5: regex engine, generators, Map/Set/WeakMap, Symbol, iterators, typed arrays, …
- Perf track: (shapes now 0083). 0081 structural proposals: P6+P9 done (0093), P8 in progress (0100), P7 (bounds checks
  per cell) open. A one-codegen-unit LTO
  profile for `bench.sh --record` (D1620: default release layouts vary +-15%).
- Perf track: the bgen Rust backend, benchmarks. **Interpreter call path is ~10x slower
  than qjs** (0034: 3.4M plain JS calls 0.6 s vs 0.06 s; sort with comparator 30x): the
  next perf lever, likely for heap after 0030.

## Scores

| date | difftest (djs) | test262 active |
| --- | --- | --- |
| 2026-11-01 | 50 / 154 (11 skipped) | 2,961 / 20,124 (14.7%) |
| 2026-11-01 (0031 part 1) | 55 / 168 | 5,108 / 20,124 (25.4%) |
| 2026-11-01 (0031 part 2) | 74 / 171 | 7,687 / 20,124 (38.2%) |
| 2026-11-01 (0029 done) | ~95 / 175 | 11,773 / 20,124 (58.5%) |
| 2026-11-01 (0038 p1: promotion to active, +2,018 tests) | | 12,354 / 22,142 (55.8%) |
| 2026-11-01 (0038 p2: Map/Set/Weak*) | | 13,049 / 22,142 (58.9%) |
| 2026-11-01 (0038 done + 0037 p1) | 96 / 182 | 14,367 / 22,142 (64.8%) |
| 2026-11-01 (0039 done; binary data promoted, +1,978 tests) | | 18,484 / 24,120 (76.6%) |
| 2026-11-01 (0041 done) | 62 expected failures | 18,833 / 24,120 (78.1%) |
| 2026-11-01 (D011: generators + async iteration promoted, +8,213 tests) | | 22,605 / 32,333 (69.9%) |
| 2026-11-01 (0040 classes done) | 52 expected failures | 31,791 / 32,718 (97.1%) |
| 2026-11-01 (0044 + 0033 done) | 38 expected failures (before 0033) | 32,303 / 32,718 (98.7%) |
| 2026-11-01 (0046 BigInt done + promoted) | | 33,450 / 33,854 (98.8%) |
| 2026-11-01 (0051 p1: iterator helpers promoted) | | 33,879 / 34,283 (98.8%) |
| 2026-11-01 (0050: RegExp promoted, +2,186 tests) | | 36,285 / 36,689 (98.9%) |
| 2026-11-01 (0051 done) | | 36,778 / 37,182 (98.9%) |
| 2026-11-01 (0059: indirect eval + Function, +540 tests) | | 37,464 / 37,722 (99.3%) |
| 2026-11-01 (0082 long tail) | 4 expected failures | 56 expected failures in active (~99.85%) |
| 2026-11-01 (0097: later group empty) | 224 / 230 | 38,069 / 38,109 (99.9%) |

## Id reservations (in flight; check main for the authoritative list)

| what | id | owner / issue |
| --- | --- | --- |
| ObjectClass RegExp, RegExpStringIterator | 25, 26 | num / 0033 |
| ObjectClass Proxy | 27 | infra / 0045 (merged) |
| ObjectClass BigInt wrapper | 30 | heap / 0046 (merged) |
| CellKind BigInt | 24 | heap / 0046 (merged) |
| ObjectClass IteratorHelper, WeakRef, FinalizationRegistry, DisposableStack | 28, 31, 32, 33 | heap / 0051 (merged) |
| ObjectClass RawJSON | 29 | num / 0061 (merged) |
| CellKind AccessorPair, Shape, PropValues | 25, 26, 27 | heap / 0083 |
| object header flag OBJ_DICTIONARY | 128 | heap / 0083 |
| ObjectClass CallSite | 34 | infra / 0106 |
| next free ObjectClass | 35 | |
| func_flags PRIVILEGED | 1 << 8 | heap / 0043 (merged) |
| func_flags HOME | 1 << 6 (check) | lexer / 0040 |

Realm `Intrinsic` enum: append at the end; rebase conflicts there are expected and trivial.

## Parallelism notes

0002, 0003, 0004 and 0005 touch disjoint areas (the only shared file is
`djs-core/src/lib.rs`, mod lines only, which rebases trivially), and none depends on
another. So they run in parallel.
