# 0012: Native JSON (leaf): parse and stringify fast path

status: in-progress
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
- [ ] `json::parse(heap, input) -> Result<Value, JsonError>` over Latin-1 and UTF-16
      strings (and UTF-8 bytes for host input, transcoding as you go). Builds objects
      (insertion-ordered properties, integer keys handled per spec order), arrays (dense),
      strings (one-byte where possible), numbers (via `crate::num`, D1041 rounding), true,
      false and null. Duplicate keys: the last one wins. Errors carry a position and
      QuickJS's SyntaxError message wording.
- [ ] Deep nesting without Rust recursion (an explicit stack), with a depth limit that
      fails cleanly.
- [ ] Allocation-heavy input works with the GC contract (allocation never collects). The
      parser runs to completion within one native op; document the memory behavior for
      huge inputs.
- [ ] `json::stringify_fast(heap, value, indent/gap, …) -> Result<StringRef, Slow |
      Error>`. It handles primitives, strings (spec escaping including lone surrogates as
      `\udXXX`), numbers (`to_string`, and non-finite → `null`), plain objects (own
      enumerable string keys in spec order, only data properties), arrays (dense and
      holes → `null`), and nested values with the `gap` indentation. It returns `Slow` on
      anything that may run user code or needs the full algorithm: accessors, a
      `toJSON` anywhere on the prototype chain, proxies and exotic objects (later),
      wrapper objects (Number/String/Boolean, which need `ToPrimitive`), BigInt, and a
      `replacer` (callers do not use the fast path then). It throws the TypeError for
      cycles (QuickJS message).
- [ ] Output is built efficiently (a growable one-byte buffer that switches to two-byte
      only when needed), not via ropes.
- [ ] Tests: unit tests, and a qjs-generated corpus (`tests/data/json/`, with a
      checked-in generator script running `~/Development/dquickts/qjs`): parse cases
      (valid ones, and invalid ones with the error message) and stringify cases (value +
      gap → output). Round trips with random data. Behavior after GC (parse, collect,
      stringify gives the same result).
- [ ] Numbers in the Log: MB/s for parse and stringify on a realistic codemode-like
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

## Follow-ups

## Result
