# 0046: BigInt

status: in-progress
agent: heap
branch: issue/0046-bigint
depends-on: 0043
touches: crates/djs-core/src/value.rs (the reserved bigint tag), crates/djs-core/src/heap/ (a BigInt cell kind, additive), crates/djs-core/src/num/ (or a new bigint/ module: arbitrary-precision arithmetic, string conversion), crates/djs-core/src/interp/ (operators on BigInt: isolated arms and slow paths), crates/djs-core/src/compiler/ (BigInt literals only: the lexer already lexes them), builtins/bigint.ts + leaves, typed arrays (BigInt64Array/BigUint64Array, DataView BigInt methods: currently stubs from 0039), wire + reader (a bigint tag, if cheap), tests/test262.toml (promote BigInt), ratchets

## Goal
BigInt is the last missing primitive type (ARCH "Values": the tag is reserved). It
blocks the BigInt test262 directory and features, the BigInt typed arrays, and a number
of difftest cases (0016: 6 BigInt literal cases).

## Acceptance criteria
- [ ] A BigInt heap cell (sign + magnitude limbs, immutable), small-value fast path if
      cheap (record a decision: for example an inline tag for |x| < 2^31, or always heap).
- [ ] Arithmetic: `+ - * / % **`, unary `-`, `~`, `& | ^ << >>` (`>>>` throws), comparisons
      with BigInt and with Number and String (exact, no float rounding), `==` with
      Number/String, `typeof`, ToBoolean, ToString/ToNumber rules (TypeError when mixing
      with Number in arithmetic, qjs messages), `BigInt()` conversion from strings and
      numbers, `asIntN`/`asUintN`, `toString(radix)`, `toLocaleString`, `valueOf`, the
      BigInt wrapper object and Symbol.toPrimitive interaction, JSON.stringify throwing
      TypeError, and Map/Set SameValueZero keys by value.
- [ ] Division and multiplication: schoolbook is fine at first, with fuel charged in
      proportion to the limb work. Note the asymptotics in the Log; huge exponents must
      hit the memory cap or fuel cleanly (`2n ** 10000000n`).
- [ ] Literals: decimal/hex/octal/binary with separators, compiled into constants.
- [ ] BigInt64Array, BigUint64Array, and DataView get/setBigInt64/BigUint64 (replacing
      the 0039 stubs; coordinate with infra via the coordinator if its code needs
      restructuring).
- [ ] Promote built-ins/BigInt and the `BigInt` feature to active; report the numbers
      before and after.
- [ ] A difftest/qjs corpus for arithmetic (randomized, like 0004's num corpus), e2e in
      the three modes, and the ratchets.

## Notes
- lexer (0040 classes) and infra (0045 Proxy) are in interp/ at the same time. Keep the
  BigInt arms isolated (a slow path, ideally a separate file), and rebase often.

## Log

## Follow-ups

## Result
