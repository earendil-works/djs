// Generates corpus.txt: number <-> string conversions as computed by QuickJS.
// Run with scripts in this directory: ./gen-corpus.sh (uses ~/Development/dquickts/qjs).
// `qjs gen-corpus.js SEED SCALE > big.txt` makes a different, larger corpus; check it with
// `DJS_NUM_CORPUS=big.txt cargo test -p djs-core qjs_corpus_external -- --ignored`.
//
// One case per line, tab separated. Doubles are given as 16 hex digits of their bits.
//   str    <bits>                    String(x)
//   radix  <bits>  <radix>           x.toString(radix)
//   fixed  <bits>  <digits>          x.toFixed(digits)
//   exp    <bits>  <digits|u>        x.toExponential(digits)  (u: undefined)
//   prec   <bits>  <precision|u>     x.toPrecision(precision)
//   num    <json string>             Number(s)
//   pf     <json string>             parseFloat(s)
//   pi     <json string>  <radix>    parseInt(s, radix)
// followed by a final column with the result: a string for formatting (or "!RangeError"),
// the result bits for parsing.

const dv = new DataView(new ArrayBuffer(8));
function bits(x) {
  dv.setFloat64(0, x);
  return dv.getUint32(0).toString(16).padStart(8, "0") + dv.getUint32(4).toString(16).padStart(8, "0");
}
function fromHalves(hi, lo) {
  dv.setUint32(0, hi >>> 0);
  dv.setUint32(4, lo >>> 0);
  return dv.getFloat64(0);
}

// Optional arguments (for larger out-of-tree runs): seed and a size multiplier.
const args = typeof scriptArgs !== "undefined" ? scriptArgs.slice(1) : [];
const SCALE = args.length > 1 ? Number(args[1]) : 1;

// mulberry32: deterministic, so the corpus only changes when this script does.
let seed = args.length > 0 ? Number(args[0]) | 0 : 0x6a09e667;
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

// ---- doubles ----
const doubles = [
  0, -0, NaN, Infinity, -Infinity, 1, -1, 0.1, 0.5, 1.5, 2.5, -2.5, 0.125, 1 / 3, 2 / 3,
  0.1 + 0.2, 1e21, 1e-7, 1e-6, 123e-20, 1.005, 1.45, 1.25, 4.35, 9.95, 99.99, 999.9,
  Number.MAX_VALUE, -Number.MAX_VALUE, Number.MIN_VALUE, -Number.MIN_VALUE,
  2.2250738585072014e-308, 2.225073858507201e-308, Number.MAX_SAFE_INTEGER,
  Number.MAX_SAFE_INTEGER + 1, Number.MAX_SAFE_INTEGER + 3, 2 ** 60, 2 ** 64, 2 ** 70, 2 ** -1074,
  2 ** -1022, 2 ** 1023, 999999999999999900000, 1e20, 123456789, 0.000001234, 1.7976931348623155e308,
];
for (let e = -1074; e <= 1023; e += 1 + below(8)) doubles.push(2 ** e);
for (let e = -323; e <= 308; e += 1 + below(6)) doubles.push(Number("1e" + e));
while (doubles.length < 2500 * SCALE) {
  let x;
  switch (below(6)) {
    case 0:
    case 1:
      x = fromHalves(rand32(), rand32());
      break;
    case 2: // short decimals
      x = below(100000) * 10 ** (below(40) - 20);
      break;
    case 3: // dyadic fractions (ties for toFixed)
      x = below(1 << 20) / 2 ** below(12);
      break;
    case 4: // integers
      x = below(2 ** 31) * (below(2) ? 1 : 2 ** below(40));
      break;
    default: // neighbors of powers of two and ten
      x = fromHalves(rand32() & 0x7ff00000, below(3)) * (below(2) ? 1 : 10 ** below(10));
      break;
  }
  if (below(2)) x = -x;
  doubles.push(x);
}

for (const x of doubles) {
  const b = bits(x);
  out("str", b, String(x));
  let radix = 2 + below(35);
  // Outside of about 1e-30..1e30 small radixes print hundreds of digits (no exponent
  // notation); keep those cases rare to keep the corpus small.
  const ax = Math.abs(x);
  if ((ax > 1e30 || (ax < 1e-30 && ax > 0)) && below(10)) radix = 36;
  out("radix", b, radix, attempt(() => x.toString(radix)));
  const fd = below(8) ? below(21) : below(101);
  out("fixed", b, fd, attempt(() => x.toFixed(fd)));
  out("exp", b, "u", attempt(() => x.toExponential()));
  const ed = below(8) ? below(21) : below(101);
  out("exp", b, ed, attempt(() => x.toExponential(ed)));
  if (below(2)) out("prec", b, "u", attempt(() => x.toPrecision()));
  const p = below(8) ? 1 + below(21) : 1 + below(100);
  out("prec", b, p, attempt(() => x.toPrecision(p)));
}
// Range errors and the argument checks' order.
for (const x of [1.5, NaN, Infinity, -0]) {
  const b = bits(x);
  for (const d of [-1, 0, 100, 101]) {
    out("fixed", b, d, attempt(() => x.toFixed(d)));
    out("exp", b, d, attempt(() => x.toExponential(d)));
    out("prec", b, d, attempt(() => x.toPrecision(d)));
  }
  for (const r of [1, 2, 36, 37]) out("radix", b, r, attempt(() => x.toString(r)));
}

// ---- strings ----
const spaces = [" ", "\t", "\n", "\v", "\f", "\r", "\u00a0", "\ufeff", "\u2028", "\u2029", "\u3000", "\u1680", "\u2000", "\u200a", "\u202f", "\u205f", "\u0085", "\u180e", "\u001c"];
const junk = ["x", "e", "E", ".", "_", "+", "-", "0x", "n", "\u0000", "Infinity", "z", "8", "9", " 1"];
function digits(n) {
  let s = "";
  for (let i = 0; i < n; i++) s += String.fromCharCode(48 + below(10));
  return s;
}
function radixDigits(n, radix) {
  let s = "";
  for (let i = 0; i < n; i++) {
    const d = below(radix).toString(36);
    s += below(2) ? d.toUpperCase() : d;
  }
  return s;
}
function decimal() {
  // At most 36 significant digits: qjs keeps 38 and truncates the rest, where djs rounds
  // correctly, so longer inputs can legitimately differ (see docs/decisions.md).
  const ni = below(4) ? below(18) : 0;
  const nf = below(3) ? below(18) : 0;
  let s = digits(ni);
  if (nf || below(3) == 0) s += ".";
  s += digits(nf);
  if (ni + nf == 0 && below(4)) s += "7";
  if (below(2)) s += pick(["e", "E"]) + pick(["", "+", "-"]) + (below(3) ? below(400) : digits(1 + below(25)));
  if (below(6) == 0) s = "0".repeat(1 + below(30)) + s;
  return s;
}
function nonDecimal() {
  const [p, radix] = pick([["0x", 16], ["0X", 16], ["0o", 8], ["0O", 8], ["0b", 2], ["0B", 2]]);
  return p + radixDigits(below(5) ? 1 + below(20) : 1 + below(300), radix);
}
function decorate(s) {
  if (below(4) == 0) s = pick(["+", "-"]) + s;
  if (below(4) == 0) s = pick(spaces) + s;
  if (below(4) == 0) s = s + pick(spaces);
  if (below(10) == 0) s = s + pick(junk);
  if (below(15) == 0) s = pick(junk) + s;
  return s;
}
const strings = [
  "", " ", "Infinity", "-Infinity", "+Infinity", "infinity", "Infinityx", "NaN", "0x", "0b", "0o",
  "-0", "+0", "-0x10", "+0x10", "1_000", "1e", "1e+", ".", ".e1", "5.", ".5", "-.5", "00012",
  "1e1000", "-1e-1000", "0e999999999999", "4.9406564584124654e-324", "2.4703282292062328e-324",
  "2.4703282292062327e-324", "1.7976931348623158e308", "1.7976931348623159e308",
  "9007199254740993", "9007199254740995", "0x1fffffffffffff1", "0x20000000000001",
];
while (strings.length < 3000 * SCALE) {
  strings.push(decorate(below(5) ? decimal() : nonDecimal()));
}
for (const s of strings) {
  const j = JSON.stringify(s);
  out("num", j, bits(Number(s)));
  out("pf", j, bits(parseFloat(s)));
  const r = pick([0, 0, 10, 16, 2, 8, 36, 1 + below(37), -1]);
  out("pi", j, r, bits(parseInt(s, r)));
}
// parseInt on digit strings in every radix, long and short.
for (let i = 0; i < 1000 * SCALE; i++) {
  const radix = 2 + below(35);
  let s = radixDigits(below(6) ? 1 + below(20) : 1 + below(400), radix);
  if (below(4) == 0) s = pick(["-", "+", " ", "-0x", "0x"]) + s;
  out("pi", JSON.stringify(s), radix, bits(parseInt(s, radix)));
}

console.log(lines.join("\n"));
