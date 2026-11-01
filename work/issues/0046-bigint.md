# 0046: BigInt

status: done
agent: heap
branch: issue/0046-bigint
depends-on: 0043
touches: crates/djs-core/src/value.rs (the reserved bigint tag), crates/djs-core/src/heap/ (a BigInt cell kind, additive), crates/djs-core/src/num/ (or a new bigint/ module: arbitrary-precision arithmetic, string conversion), crates/djs-core/src/interp/ (operators on BigInt: isolated arms and slow paths), crates/djs-core/src/compiler/ (BigInt literals only: the lexer already lexes them), builtins/bigint.ts + leaves, typed arrays (BigInt64Array/BigUint64Array, DataView BigInt methods: currently stubs from 0039), wire + reader (a bigint tag, if cheap), tests/test262.toml (promote BigInt), ratchets

## Goal
BigInt is the last missing primitive type (ARCH "Values": the tag is reserved). It
blocks the BigInt test262 directory and features, the BigInt typed arrays, and a number
of difftest cases (0016: 6 BigInt literal cases).

## Acceptance criteria
- [x] A BigInt heap cell (sign + magnitude limbs, immutable), small-value fast path if
      cheap (record a decision: for example an inline tag for |x| < 2^31, or always heap).
- [x] Arithmetic: `+ - * / % **`, unary `-`, `~`, `& | ^ << >>` (`>>>` throws), comparisons
      with BigInt and with Number and String (exact, no float rounding), `==` with
      Number/String, `typeof`, ToBoolean, ToString/ToNumber rules (TypeError when mixing
      with Number in arithmetic, qjs messages), `BigInt()` conversion from strings and
      numbers, `asIntN`/`asUintN`, `toString(radix)`, `toLocaleString`, `valueOf`, the
      BigInt wrapper object and Symbol.toPrimitive interaction, JSON.stringify throwing
      TypeError, and Map/Set SameValueZero keys by value.
- [x] Division and multiplication: schoolbook is fine at first, with fuel charged in
      proportion to the limb work. Note the asymptotics in the Log; huge exponents must
      hit the memory cap or fuel cleanly (`2n ** 10000000n`).
- [x] Literals: decimal/hex/octal/binary with separators, compiled into constants.
- [x] BigInt64Array, BigUint64Array, and DataView get/setBigInt64/BigUint64 (replacing
      the 0039 stubs; coordinate with infra via the coordinator if its code needs
      restructuring).
- [x] Promote built-ins/BigInt and the `BigInt` feature to active; report the numbers
      before and after.
- [x] A difftest/qjs corpus for arithmetic (randomized, like 0004's num corpus), e2e in
      the three modes, and the ratchets.

## Notes
- lexer (0040 classes) and infra (0045 Proxy) are in interp/ at the same time. Keep the
  BigInt arms isolated (a slow path, ideally a separate file), and rebase often.

## Log
- First, the 0043 follow-up (coordinator): `Vm::collect` clears the value stack above
  `sp` like the interpreter's safepoint, so an embedder's collection (`vm_collect`) no
  longer keeps popped values alive; test `collect_drops_popped_stack_values` (fails
  without the fix: 365 KB -> 589 KB kept).

- test262 baseline (built-ins/BigInt + feature BigInt, 1142 tests, measured with a
  scratch group): djs 161 pass (14.0%), qjs 1142 (100%). Failures: 638 BigInt literals
  unsupported, 258+ BigInt typed arrays, 26 `BigInt` not defined.
- Part 1: D1460 (always-heap cell kind 24, ObjectClass 30, compared by value), D1461
  (`crate::bigint`: schoolbook/Knuth D, 2^20-bit cap, fuel after the fact), D1462 (slow
  path beside exec, qjs messages). Literals as constants (`{1n: x}` keys too), operators,
  comparisons/equality with Number and String, typeof/ToBoolean/ToString, `BigInt()`,
  asIntN/asUintN, toString(radix)/toLocaleString/valueOf, the wrapper (ToPrimitive works
  through the generic path), JSON.stringify TypeError (and BigInt.prototype.toJSON),
  Number(bigint), SameValue/Map/Set keys by value. My qjs probe scripts (~150 cases)
  match except toLocaleString("de"), where qjs throws (uses the locale as a radix).
- Asymptotics: add/sub/bitwise/shifts O(n); mul O(n m); div O((n-m) m); pow O(r^2) for
  an r-limb result (squarings dominate); toString O(n^2) (O(n) for power-of-two radixes);
  parse O(n^2). With the 32768-limb cap the worst single operation is ~2.7e8 limb
  products (16k x 16k), uninterruptible but charged as fuel afterwards.

- JSON fast path: `StringifyError::BigInt` (TypeError) when no toJSON is on the BigInt's
  prototype chain, so hosts (difftest glue, djs-wasm) report qjs's error;
  `JSON.stringify` itself falls back to the slow path. `djs run`'s console prints `10n`
  like qjs (e2e prelude in gen-expected.sh too).
- qjs bug (node as tie-breaker): qjs compares a negative BigInt with a negative
  non-integral Number wrongly (`-1n < -0.5` is false in qjs, true in node and the spec).
  djs follows the spec; e2e and the corpus avoid that case.
- e2e `bigint.js` (three modes); test262 active +29 (tests that used BigInt without the
  feature tag); difftest ratchet: test_map.js and result/bigint.js pass now.

- Part 2 (typed arrays, D1463; coordinator OK'd touching infra's typed-array code).
  For infra, the new signatures:
  - heap/typed.rs: `Elem { Number(f64), Big(u64) }`, `Kind::is_bigint`,
    `encode_elem`/`decode_elem`/`elem_bigint`; `ta_elem(&self) -> Option<Elem>`;
    `ta_get(&self)` gives `Value::HOLE` for a BigInt element (placeholder), use
    `ta_get_alloc(&mut self) -> Result<Option<Value>>` to hand values to JS;
    `ta_set(o, i, Elem)` (was f64); `element_of(kind, v) -> Option<Elem>` replaces
    `primitive_number` in the write paths; `get_own_property_alloc`,
    `find_property_alloc`, `materialize_value` for reads that expose values.
  - builtins/typed.rs: `view_get` returns `Elem`, `view_set` takes `Elem`;
    `typed_array_copy` copies `Elem`s.
  - leaves: `TypedArraySet(o, i, v: Value)`, `DataViewGet(...): Value`,
    `DataViewSet(view, at, kind, v: Value, little)`; `SortNumbers` sorts BigInts too.
  - typedarray.ts: `ToElementValue`/`IsElementValue`/`SameContentType` (ToBigInt for
    BigInt arrays) replace the ToNumber calls; `RequireNumberKind` is gone; species
    results must have the same content type. object.ts DefineOwnProperty and reflect.ts
    Reflect.set use IsElementValue/ToElementValue.
  - interp: PutElem/PutField on a BigInt typed array calls `%ToBigInt` (generalized
    `call_converter(id, slot)`, was `call_to_number`).
- test262 BigInt group (built-ins/BigInt + feature BigInt, 1142 tests): before 161
  (14.0%), after 1121 (98.1%). The 21 left: classes 9, regexp 6, Promise.allKeyed 2,
  JSON.rawJSON, Error.isError, Array.fromAsync, harness deepEqual (regexp). Promoted to
  active: active 23,007 / 32,718 (70.3%) -> 24,139 / 33,854 (71.3%).

- Corpus: `tests/data/bigint/gen-corpus.js` (qjs, seeded, ~19k cases: all operators,
  asIntN/asUintN, toString radixes, StringToBigInt, Number <-> BigInt, exact compares)
  checked by `bigint/corpus_tests.rs`; difftest case `language/bigint-ops.js`
  (pseudo-random operands, also in the stress modes); e2e `bigint_typed.js`. qjs's
  negative-BigInt-vs-negative-Number compare bug is broader than fractions
  (`-1n < -(2**63)` is true in qjs): the corpus skips all negative/negative compares.
- Reader: a BigInt reads as a native `bigint` (js/src/reader.ts, layout flag
  `BIGINT_NEG` exported); reader-cross extra cases. docs/wire.md updated.
- Size: wasm code +34.7 KB (614,735 -> 649,477 against 7427f4f; `bigint::BigInt` 14.6
  KB of it), base image data +6.4 KB.
- Timing (native release, `djs run`): `3n ** 600000n` 0.23 s, a 16k x 16k limb product
  0.19 s, a 32k / 16k limb division 0.25 s, parsing 300k decimal digits 0.25 s; the
  decimal toString of a 1 Mi-bit value 1.7 s (qjs 1.26 s): the quadratic radix
  conversion is the slowest operation. All of them run to completion, then pay fuel.

## Follow-ups
- Decimal conversion: divide and conquer (or at least 10^18 chunks with u64 math) would
  cut the 1.7 s worst case; Karatsuba for very large products. Neither matters for
  codemode-sized values.
- The inbound wire format (host -> VM) has no BigInt tag; `encode` still rejects
  bigints with a TypeError.
- Builtins' generic `Set(o, k, v)` on a typed array (Array.prototype methods called on a
  typed array) writes nothing when v needs a conversion with user code, for Number
  arrays too (pre-existing in `ordinary_set`); BigInt arrays add ToBigInt of numbers.
- A small inline BigInt form (tag 7) only if benchmarks ever show BigInt allocation.

## Result
BigInt is a full primitive. Values are always-heap `BigInt` cells (kind 24) compared by
value (D1460); `crate::bigint` does the arithmetic with QuickJS's 2^20-bit cap and its
RangeErrors (D1461); operators run in a slow path beside `exec` with qjs's messages
(D1462); literals (all radixes, separators) are constants; `BigInt()`, asIntN/asUintN,
toString/toLocaleString/valueOf, the wrapper and Symbol.toPrimitive, JSON.stringify's
TypeError (and toJSON), Number(bigint), exact comparisons and `==` with Number/String,
Map/Set/SameValue by value. BigInt64Array/BigUint64Array and DataView
get/setBigInt64/BigUint64 replace the 0039 stubs (D1463; new heap read/write signatures
in the Log). `djs run` prints `10n`; the JS reader returns native bigints.

test262: the BigInt group (built-ins/BigInt + feature BigInt, 1142 tests) went from 161
(14.0%) to 1121 (98.1%), and to 1135 (99.3%) after rebasing onto 0040's classes; it is
promoted to active. Active after the final rebase: 33,450 / 33,854 (98.8%, 404 expected
failures). The 7 left fail on regexp and unrelated proposals.

Known gaps: see Follow-ups (radix conversion speed, inbound wire tag, builtin Set on
typed arrays). `BigInt.prototype.toLocaleString(locale)` ignores the locale like node
(qjs throws).
