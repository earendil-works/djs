# Builtin families: what leaving each out of the shipped module gives and costs

Issue 0283, measured at main 7b302395 (2026-11-21) for the owner's decision. Scripts are
in this directory. Load was 20–80 while other agents built. The numbers below are sizes,
wasm memory pages and instruction counts, which don't depend on load; no wall times are
used.

## In short

- **An idle sandbox reaches QuickJS's 320 KiB only by leaving families out.** Its arena
  must fit 2 pages (≤ 131,072 B). Today it is 185,704 B: the base image (184,400) plus
  1,304 B of the sandbox's own cells. No single family is enough. The smallest packages
  that cross the line are below.
- **Real codemode scripts use none of these families.** 0 of 2,535 scripts (0130).
- **pi itself needs three things:**
  - Proxy: `tools` is a Proxy.
  - Reflect: pi's prelude and djs's dump runtime use it.
  - An `Int8Array`, because pi's lockdown freezes `%TypedArray%`.

  Everything else can go without breaking pi.
- **Recommended:** **"idle-320"** (package B). Idle 384 → 320 KiB, five codemode rows
  −64 KiB, module about −35 KB gzip, pi unchanged apart from an `Int8Array` stub. It
  costs 4,864 test262 tests and the families' availability.

## Per family

Columns:
- **base**: bytes the base image (the realm every VM copies) loses when the family's
  builtins file is left out of the prelude.
- **module, gz**: the base section measured; the family's Rust, which would become dead
  code, estimated from function sizes in a names build. The slim build confirms the
  total for the families it already leaves out (−19.0 KB gz of Rust).
- **test262**: active tests that touch the family. They are counted by directory,
  `features:` front matter or the syntax; they overlap.

| family | base | module, gz (base + Rust) | test262 | scripts (0130) | pi | what a script sees | lazy chunk instead |
| --- | ---: | ---: | ---: | ---: | --- | --- | --- |
| typed arrays, ArrayBuffer, DataView (+ base64/hex) | 25.5 KB | 5.3 + ~6 | 3,071 | 0 | lockdown reads `Object.getPrototypeOf(Int8Array)`: needs a stub `%TypedArray%` + `Int8Array` that throw (~0.6 KB), or a one-line `typeof` guard in pi; byte values (`Uint8Array`) can't come in over the wire | ReferenceError: 'Uint8Array' is not available in this build | yes (global-only; the wire loads it for bytes) |
| Iterator helpers (`Iterator`, `.map/.filter/...` on iterators) | 11.0 KB | 3.3 + ~0 | 657 | 0 | guarded (`typeof Iterator`) | `typeof Iterator` is "undefined"; `it.map()`: TypeError: not a function | no: the methods sit on the shared iterator prototype; stubs only |
| Proxy | 7.6 KB | 2.1 + ~1.7 | 449 | 0 | **breaks pi**: `tools` is a Proxy | ReferenceError | yes |
| Reflect | 2.1 KB | 0.5 | 947 | 0 | **breaks pi** (prelude) and dumps (the dump runtime) | ReferenceError | yes |
| DisposableStack, AsyncDisposableStack, SuppressedError, `using` | 6.6 KB | 1.6 + ~1 | 414 | 0 | — | SyntaxError: `using` declarations are not available in this build | globals yes; `using` loads it at compile |
| decorators | 4.2 KB | 1.3 + ~4 | 18 | 0 (no classes at all) | — | SyntaxError: decorators are not available in this build | loads at compile |
| web globals: `DOMException`, `atob`, `btoa` | 6.0 KB | 1.3 | 0 (not ECMAScript) | 0 | `atob`/`btoa` were given to scripts by pi's QuickJS host; pi could keep them as host globals | ReferenceError | yes |
| WeakMap, WeakSet | 2.2 KB | 0.35 | 266 | 0 | — | ReferenceError | yes |
| WeakRef, FinalizationRegistry | 1.5 KB | 0.35 + ~1.5 | 80 | 0 | pi's QuickJS has neither; djs's adapter already deletes them in deterministic mode | ReferenceError | yes |
| `Array.fromAsync` | 2.1 KB | 0.6 | 93 | 0 | — | TypeError: not a function | no (a method of `Array`) |
| BigInt (global, literals) | 1.3 KB | 0.3 + ~7.5 | 1,247 | 0 | — | SyntaxError: BigInt literals are not available in this build | global yes; literals load it at compile |
| URI functions (`encodeURI*`, `decodeURI*`, `escape`, `unescape`) | 0.8 KB | 0.1 + ~1.7 | 173 | 0 | — | ReferenceError | yes |

**Not separable:**
- Symbol: protocols, pi.
- Generators and async generators: 1.8 + 3.1 KB, but pi's prelude uses the syntax.
- Date: 43 scripts.
- `eval`/`Function`: 0.5 KB, in one file with `queueMicrotask`.

**Startup (new sandbox):** ~1.3M instructions today, of which copying the base is a few
percent. A package changes it by −2% (slim set) to −6% (B, C), a few µs.

## Per-VM memory: which combinations cross a page

Each family alone moves no idle page: the largest, typed arrays, still leaves 160 KB.
Shipped-module builds with the families left out of the prelude, the arena and pages of
an idle sandbox, and the codemode rows (KiB; all results identical to main's):

| package | idle arena | idle | seq | fanout | media | inspect | loop | text | json | heavy |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| main | 185,704 | 384 | 512 | 1472 | 768 | 448 | 640 | 896 | 1216 | 3968 |
| slim set (typed, Iterator, using, decorators, WeakRef, BigInt) | 135,544 | 384 | 448 | 1536 | 768 | 384 | 640 | 896 | 1152 | 3904 |
| A: slim set + fromAsync + URI | 132,656 | 384 | 448 | 1536 | 768 | 384 | 640 | 896 | 1152 | 3904 |
| **B: A + web globals** | **126,392** | **320** | 448 | 1536 | 704 | 384 | 640 | 896 | 1152 | 3904 |
| A + WeakMap/WeakSet (instead of web) | 130,312 | 320 | 448 | 1536 | 768 | 384 | 640 | 896 | 1152 | 3904 |
| C: B + WeakMap/WeakSet + Proxy + Reflect | 114,320 | 320 | 448 | 1536 | 704 | 384 | 576 | 896 | 1152 | 3904 |

- The packages above are built without the `Int8Array` stub; it would add ~0.6 KB. With
  it, B keeps ~4 KB of margin under 131,072. "A + WeakMap/WeakSet" keeps 0.2 KB, too thin.
- **fanout +64 KiB** in every package is the arena's growth path: from 9 pages it grows 2
  at a time. pgc has the fix (rounding the step down), which also brings text −64.

## Packages

| | A: pi-safe minimum | **B: idle-320** | C: max |
| --- | --- | --- | --- |
| leaves out | typed arrays (stub kept), Iterator helpers, `using`/DisposableStack, decorators, WeakRef/FR, BigInt, `Array.fromAsync`, URI functions | A + `DOMException`, `atob`, `btoa` | B + WeakMap/WeakSet, Proxy, Reflect |
| base image | −52 KB | −58 KB | −70 KB |
| idle sandbox | 384 KiB (1.6 KB short) | **320 KiB** | 320 KiB |
| codemode rows | seq, inspect, json, heavy −64; fanout +64 | + idle, media −64 | + loop −64 |
| module | ~−33 KB gz (~−67 KB raw) | ~−35 KB gz | ~−40 KB gz |
| test262 tests lost | 4,864 | 4,864 | 6,095 |
| pi | stub `Int8Array` (or pi adds a guard) | same; pi may provide `atob`/`btoa` itself | **breaks pi**: `tools` Proxy, Reflect in the prelude and in dumps |

**Module bytes.** The base section measures the same raw and gzipped (it ships coded).
The Rust part needs those families behind cargo features, as the slim build has them:
- A and B: the slim set's feature (it exists) plus small gates for `fromAsync` and URI.
- C additionally: gates for Proxy's traps. The checks for proxies in the object paths
  stay.

## The lazy-chunk alternative

Keep the families but load each as a second image chunk on first use:
- **Mechanism:** one mechanism for all of them, 2–3 weeks. It covers relocation, the
  registry and the dumper for chunk cells, and frozen-builtins mode for cells above the
  base. The loader adds ~2–4 KB gz.
- **What it saves:** the same per-VM memory as leaving a family out, until a script uses
  it. No module bytes: the chunks ship. First use costs a chunk load, estimated at tens
  of µs.
- **Where it applies:** families reached only through a global name (typed arrays,
  Proxy, Reflect, WeakMap/WeakSet, WeakRef/FR, web globals, URI, BigInt's global). For
  syntax families (`using`, decorators, BigInt literals) the compiler loads them.
- **Where it doesn't:** Iterator helpers and `Array.fromAsync`, whose methods hang off
  prototypes every VM shares. They would stay in the base or become stubs, so a lazy
  version of B saves ~13 KB less and doesn't reach 320 KiB on its own.

## Method

- **Base sizes:** `famvar.sh`. The full base built natively with one builtins file left
  out of the prelude (bgen on a copy of builtins/).
- **Module sizes, idle arena and rows:** `famrun.sh`, `rows.ts`, `idle.ts`. The shipped
  module built with that prelude.
- **Rust estimates:** function sizes of a names build, by module. The slim build's
  measured delta (full vs slim, both without the Unicode features: −61.6 KB raw,
  −31.4 KB gz, of which −12.4 KB is base) gives −19.0 KB gz of Rust for the slim set.
- **test262:** `t262.py` over the active group's paths.
- **Startup:** `icount.sh` (instructions per new sandbox, 2,000 sandboxes, min of 3).
- **What a script sees:** the slim module and the package builds. Families outside
  today's slim set print "is not defined" until their names join the "not available in
  this build" list.
