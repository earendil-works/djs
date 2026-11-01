// Generates corpus.txt: BigInt arithmetic and conversions as computed by QuickJS (0046).
// Run ./gen-corpus.sh (uses ~/Development/dquickts/qjs). Checked by
// crates/djs-core/src/bigint/corpus_tests.rs.
//
// One case per line, tab separated: an operation, its operands, the result. BigInts are
// written in hex with a sign (`-1f`, `0`); doubles as 16 hex digits of their bits;
// errors as `!Name`.
//   add sub mul div mod and or xor shl sar pow   <a> <b>    a op b
//   cmp     <a> <b>        -1, 0 or 1
//   not neg <a>            ~a, -a
//   intn uintn <bits> <a>  BigInt.asIntN / asUintN
//   str     <a> <radix>    a.toString(radix)
//   parse   <json string>  BigInt(s): hex result or !SyntaxError
//   tonum   <a>            Number(a) as bits
//   fromnum <bits>         BigInt(x) as hex, or !RangeError
//   cmpnum  <a> <bits>     -1, 0, 1 or u (NaN)

// mulberry32: deterministic, so the corpus only changes when this script does.
let seed = 0x3c6ef372;
function rand32() {
  seed = (seed + 0x6d2b79f5) | 0;
  let t = seed;
  t = Math.imul(t ^ (t >>> 15), t | 1);
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
  return (t ^ (t >>> 14)) >>> 0;
}
function below(n) {
  return rand32() % n;
}
function pick(a) {
  return a[below(a.length)];
}
const dv = new DataView(new ArrayBuffer(8));
function bits(x) {
  dv.setFloat64(0, x);
  return dv.getUint32(0).toString(16).padStart(8, "0") + dv.getUint32(4).toString(16).padStart(8, "0");
}
function fromBits(hi, lo) {
  dv.setUint32(0, hi >>> 0);
  dv.setUint32(4, lo >>> 0);
  return dv.getFloat64(0);
}
function hex(b) {
  return b < 0n ? "-" + (-b).toString(16) : b.toString(16);
}
const lines = [];
function out(...cols) {
  lines.push(cols.join("\t"));
}
function attempt(f) {
  try {
    return f();
  } catch (e) {
    return "!" + e.name;
  }
}

// A random BigInt of up to `limbs` 32-bit limbs, biased towards edges: all-ones and
// zero limbs, powers of two and their neighbours.
function big(limbs) {
  const n = below(limbs + 1);
  let v = 0n;
  const style = below(6);
  for (let i = 0; i < n; i++) {
    let l;
    if (style === 0) l = 0xffffffff;
    else if (style === 1) l = i === n - 1 ? 1 : 0;
    else l = pick([0, 0xffffffff, rand32(), rand32(), rand32(), 0x80000000, 1]);
    v = (v << 32n) | BigInt(l);
  }
  if (style === 1 && below(2)) v -= 1n;
  if (below(2)) v = -v;
  return v;
}
function small() {
  return BigInt(below(2) ? below(70) : below(400)) - (below(4) === 0 ? 200n : 0n);
}

const binops = {
  add: (a, b) => a + b,
  sub: (a, b) => a - b,
  mul: (a, b) => a * b,
  div: (a, b) => a / b,
  mod: (a, b) => a % b,
  and: (a, b) => a & b,
  or: (a, b) => a | b,
  xor: (a, b) => a ^ b,
};
for (let i = 0; i < 700; i++) {
  const size = pick([2, 3, 5, 12, 24]);
  const a = big(size);
  const b = below(3) === 0 ? big(pick([1, 2, 3])) : big(size);
  for (const [name, f] of Object.entries(binops)) {
    const r = attempt(() => hex(f(a, b)));
    out(name, hex(a), hex(b), r);
  }
  out("cmp", hex(a), hex(b), a < b ? -1 : a > b ? 1 : 0);
  out("cmp", hex(a), hex(a), 0);
  out("not", hex(a), hex(~a));
  out("neg", hex(a), hex(-a));
  const s = small();
  out("shl", hex(a), hex(s), attempt(() => hex(a << s)));
  out("sar", hex(a), hex(s), attempt(() => hex(a >> s)));
  const bitsN = pick([0, 1, 7, 8, 31, 32, 33, 63, 64, 65, 100, 200]);
  out("intn", bitsN, hex(a), hex(BigInt.asIntN(bitsN, a)));
  out("uintn", bitsN, hex(a), hex(BigInt.asUintN(bitsN, a)));
  const radix = pick([2, 3, 7, 8, 10, 10, 16, 32, 36]);
  out("str", hex(a), radix, JSON.stringify(a.toString(radix)));
  out("tonum", hex(a), bits(Number(a)));
}
// Division by small and by large divisors, both signs, with the identity in the oracle.
for (let i = 0; i < 500; i++) {
  const a = big(pick([4, 16, 40]));
  const b = big(pick([1, 2, 3, 8, 20]));
  out("div", hex(a), hex(b), attempt(() => hex(a / b)));
  out("mod", hex(a), hex(b), attempt(() => hex(a % b)));
  out("mul", hex(a), hex(b), hex(a * b));
}
for (let i = 0; i < 400; i++) {
  const a = big(pick([1, 2, 4]));
  const e = BigInt(below(3) === 0 ? below(200) : below(20)) - (below(10) === 0 ? 1n : 0n);
  out("pow", hex(a), hex(e), attempt(() => hex(a ** e)));
}
// Strings: valid and invalid integer literals.
const ws = ["", " ", "\t", "\n", "\u00a0", "\ufeff", "\u2028"];
for (let i = 0; i < 600; i++) {
  const a = big(pick([1, 2, 6]));
  const radix = pick([10, 10, 16, 8, 2]);
  const prefix = { 10: "", 16: pick(["0x", "0X"]), 8: pick(["0o", "0O"]), 2: pick(["0b", "0B"]) }[radix];
  let digits = (a < 0n ? -a : a).toString(radix);
  if (radix === 16 && below(2)) digits = digits.toUpperCase();
  let sign = a < 0n ? "-" : pick(["", "", "+"]);
  if (radix !== 10 && below(3)) sign = "";
  let s = pick(ws) + sign + prefix + digits + pick(ws);
  if (below(8) === 0) s = pick(["", " ", "1_000", "1e3", "1.5", "0x", "-0x1", "12n", "--1", "+", "0xg", "١", "0b2", "0o8", ".5", "5.", "Infinity"]);
  out("parse", JSON.stringify(s), attempt(() => hex(BigInt(s))));
}
// Number conversions and exact comparisons.
function randDouble() {
  switch (below(6)) {
    case 0:
      return pick([0, -0, 1, -1, 0.5, -0.5, 2 ** 53, 2 ** 53 + 2, -(2 ** 63), 2 ** 64, 1e21, 1.7976931348623157e308, NaN, Infinity, -Infinity, 5e-324]);
    case 1:
      return (below(2) ? -1 : 1) * below(1 << 30) / pick([1, 2, 4, 1024]);
    default:
      return fromBits(rand32(), rand32());
  }
}
for (let i = 0; i < 1500; i++) {
  const x = randDouble();
  out("fromnum", bits(x), attempt(() => hex(BigInt(x))));
  const a = below(2) ? big(pick([1, 2, 3, 40])) : attempt(() => BigInt(Math.trunc(x))) ;
  const av = typeof a === "bigint" ? a + BigInt(below(3)) - 1n : 0n;
  // qjs compares a negative BigInt with a negative Number wrongly unless they are close
  // (`-1n < -(2**63)` is true there); node and djs follow the spec. Skip those.
  if (av < 0n && x < 0) continue;
  out("cmpnum", hex(av), bits(x), Number.isNaN(x) ? "u" : av < x ? -1 : av > x ? 1 : 0);
  out("tonum", hex(av), bits(Number(av)));
}
print(lines.join("\n"));
