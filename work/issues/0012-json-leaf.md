# 0012: Native JSON (leaf): parse and stringify fast path

status: done
agent: heap
branch: issue/0012-json-leaf
depends-on: 0004, 0009
touches: crates/djs-core/src/json/, crates/djs-core/src/heap/ (small additions only, listed in the Log), tests/data/json/, crates/djs-core/src/lib.rs (mod lines only)

## Goal
The leaf parts of JSON (ARCH.md "Builtins: the leaf rule" and "Data passing (G1)"):
`JSON.parse` without a reviver straight into heap values, and a `JSON.stringify` fast
path for plain data that returns a "needs slow path" signal for anything that could run
user code. Codemode moves a lot of data through JSON, and host-side `JSON.stringify` +
VM `JSON.parse` is the baseline the wire format has to beat, so speed matters.

## Acceptance criteria
- [x] `json::parse(heap, input) -> Result<Value, JsonError>` over Latin-1 and UTF-16
      strings (and UTF-8 bytes for host input, transcoding as you go). Builds objects
      (insertion-ordered properties, integer keys handled per spec order), arrays (dense),
      strings (one-byte where possible), numbers (via `crate::num`, D1041 rounding), true,
      false and null. Duplicate keys: the last one wins. Errors carry a position and
      QuickJS's SyntaxError message wording.
- [x] Deep nesting without Rust recursion (an explicit stack), with a depth limit that
      fails cleanly.
- [x] Allocation-heavy input works with the GC contract (allocation never collects). The
      parser runs to completion within one native op; document the memory behavior for
      huge inputs.
- [x] `json::stringify_fast(heap, value, indent/gap, …) -> Result<StringRef, Slow |
      Error>`. It handles primitives, strings (spec escaping including lone surrogates as
      `\udXXX`), numbers (`to_string`, and non-finite → `null`), plain objects (own
      enumerable string keys in spec order, only data properties), arrays (dense and
      holes → `null`), and nested values with the `gap` indentation. It returns `Slow` on
      anything that may run user code or needs the full algorithm: accessors, a
      `toJSON` anywhere on the prototype chain, proxies and exotic objects (later),
      wrapper objects (Number/String/Boolean, which need `ToPrimitive`), BigInt, and a
      `replacer` (callers do not use the fast path then). It throws the TypeError for
      cycles (QuickJS message).
- [x] Output is built efficiently (a growable one-byte buffer that switches to two-byte
      only when needed), not via ropes.
- [x] Tests: unit tests, and a qjs-generated corpus (`tests/data/json/`, with a
      checked-in generator script running `~/Development/dquickts/qjs`): parse cases
      (valid ones, and invalid ones with the error message) and stringify cases (value +
      gap → output). Round trips with random data. Behavior after GC (parse, collect,
      stringify gives the same result).
- [x] Numbers in the Log: MB/s for parse and stringify on a realistic codemode-like
      payload (for example an array of 10k similar objects, and a 1 MiB string), native
      release. For comparison, node's `JSON.parse` / `JSON.stringify` on the same
      payload.

## Notes
- No interpreter exists yet, so test through the heap API directly. The interpreter
  (0006) will wire these into `JSON.parse` / `JSON.stringify` later, together with the
  slow-path builtins.
- Property key interning: object keys go through atoms. Make the repeated-key case fast
  (for example cache the last N key strings to atom ids during one parse, since arrays
  of similar objects repeat keys constantly).
- If you need new heap helpers (for example a bulk "create object with these
  properties" call), add them in `heap/` and list them in the Log. 0006 may start while
  you work and will also touch `heap/` (new cell kinds), so keep the changes small.

## Log
- 2026-11-01: Read QuickJS' `json_next_token`, `json_parse_string`, `json_parse_number`,
  `json_parse_value` and `js_parse_error_v` to match its messages and positions: line and
  column (1-based; lines end at `\n`; columns count code points), relative to the token
  start or the offending character. qjs tokenizes ASCII identifier words (`tru` gives
  `unexpected token: 'tru'`). After `-` at the end it prints a truncated
  `Unexpected token '` (a `%c` of NUL).
- Implemented `json/parse.rs`: a monomorphized lexer over Latin-1 / UTF-16 / UTF-8. Heap
  input is parsed in place: the arena slice is re-borrowed after every allocation, since the
  arena may reallocate. Strings are scanned once (length, width, escapes), then decoded
  straight into a fresh flat string, with bulk copies when there are no escapes. Keys go
  through a 64-entry direct-mapped cache (raw bytes to atom), then `atom_utf16`. Containers
  use an explicit stack and are built in one allocation when they close.
- Implemented `json/stringify.rs`: an explicit-stack SerializeJSONProperty in spec order.
  It bails out to `Slow` as listed in D1120, throws `circular reference`, and builds the
  output in a one-byte buffer that widens on the first unit above 0xFF.
- Heap additions (small, all additive):
  - `object.rs`: `new_object_from_entries`, `new_array_from_values`, `plain_entry_slots`,
    `plain_entry`
  - `atom.rs`: `atom_utf16`
  - `string.rs`: `alloc_str_uninit`, `str_data_offset`
- Corpus: `tests/data/json/gen-corpus.{js,sh}` writes `corpus.txt` from qjs: 1718 cases.
  Parse cases are hand-picked edge cases, 300 numbers, 300 random documents and 600 random
  mutations of them (for error messages and positions). There are also 380 stringify
  cases with numeric and string gaps. All pass through all three input encodings. A
  deliberately broken message makes 75 cases fail, so the test does check.
- Performance work: walking plain objects' tables directly (no key list, no lookups), a
  cache of `toJSON`-free prototypes, cached quoted keys, and a SWAR scan for escapes.
  Together they took records stringify from 505 to about 820 MB/s.
- Numbers (native release, Apple M5 Max;
  `cargo test -p djs-core --release json_throughput -- --ignored --nocapture`; node
  v25.8.1 on the same payloads, script in `work/scratch/0012/bench.js`):

  | payload | djs parse | djs parse (UTF-8 host) | djs stringify | node parse | node stringify |
  | --- | --- | --- | --- | --- | --- |
  | 10k similar objects, 1.52 MB (4.3 MB heap) | 460 MB/s | 450 MB/s | 820 MB/s | 600 MB/s | 1670 MB/s |
  | one 1.5 MB string with escapes | 1110 MB/s | 1010 MB/s | 1800 MB/s | 1340 MB/s | 2100 MB/s |

  Parse is at about 75-85% of V8. Records stringify is about half of V8. The rest is
  per-value heap access overhead (bounds-checked reads, kind decoding), the output buffer
  growth, and the final copy into the heap. That is good enough for a leaf. Sample profile
  in the log history: `quote`, `emit`/`member` and memmove dominate.

## Follow-ups
- Fuel: parse and stringify are long leaf ops that run to completion. 0006 should charge
  fuel proportional to the input/output size (ARCH "Long leaf ops"), and maybe add a
  resumable state later.
- The `toJSON` and class checks must be revisited when functions, wrapper objects
  (Number/String/Boolean/BigInt), proxies and other exotic classes exist. Today any
  non-ordinary, non-array class goes to the slow path, and any object-valued `toJSON` too.
- The slow paths (reviver, replacer, `toJSON`, getters) belong in the builtin language,
  0006 or later. They should take over from the fast path's bail-out point semantics
  (rerun from the start).
- Stringify speed: write the output straight into an arena cell instead of a Rust `Vec`
  plus a copy, and cut per-value overhead (fewer header reads per string).
- `parse` holds pending values in Rust vectors (8-12 bytes each); for a top-level array
  of 10M elements that is 80+ MB of transient host memory outside the cap.

## Result
- `djs_core::json::parse(heap, str, &ParseOptions) -> Result<Value, ParseError>` and
  `parse_utf8(heap, &[u8], &opts)`. `ParseOptions` carries the object and array
  prototypes (the realm's, once there is one) and `max_depth` (default 5000).
  `ParseError::Syntax(SyntaxError { message, offset, line, column })` is a JS SyntaxError
  with QuickJS wording; `ParseError::Heap` is OOM or string too long.
- `json::stringify_fast(heap, value, gap) -> Result<Option<Str>, StringifyError>`, where
  `None` is `undefined`. `StringifyError::Slow` means: run the full algorithm.
  `Circular` / `StackOverflow` / `Heap` are final; `js_error()` gives the constructor
  name and message. `gap_from_number` / `gap_from_units` compute the gap from `space`; the
  caller does any `ToNumber`/`ToString` on a `space` object first.
- Heap helpers: listed in the Log.
- Corpus: `tests/data/json/` (regenerate with `./gen-corpus.sh`).
- Known gaps: no fuel accounting. The slow paths are not here. Stringify reaches about
  half of V8's speed on object-heavy data.
