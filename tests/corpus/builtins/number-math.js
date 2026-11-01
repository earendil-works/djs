// Number, the global number functions and Math.
console.log((255).toString(16), (255).toString(2), (-255).toString(36), (0.5).toString(2), (1e21).toString());
console.log((1.005).toFixed(2), (123.456).toFixed(1), (0).toFixed(2), (1e21).toFixed(2), (-1.5).toFixed(0));
console.log((123456).toExponential(2), (0.00015).toExponential(), (1).toExponential(0));
console.log((123.456).toPrecision(4), (0.000123).toPrecision(2), (123456).toPrecision(2), (5).toPrecision());
console.log(Number("42"), Number(""), Number("0x10"), Number(" 12 "), Number("1e3"), Number(null), Number(undefined), Number(true), Number());
console.log(Number.isInteger(5), Number.isInteger(5.5), Number.isSafeInteger(2 ** 53), Number.isFinite("5"), isFinite("5"), Number.isNaN("x"), isNaN("x"));
console.log(Number.MAX_SAFE_INTEGER, Number.MIN_SAFE_INTEGER, Number.EPSILON, Number.MAX_VALUE, Number.MIN_VALUE);
console.log(parseInt("42px"), parseInt("0x1f"), parseInt("1010", 2), parseInt("z", 36), parseInt(""), parseInt("  -7"), parseFloat("3.25e2xyz"), parseFloat(".5"), parseFloat("-Infinityx"));
console.log(Number.parseInt === parseInt, Number.parseFloat === parseFloat, parseInt.length, parseFloat.name);
const n = new Number(7);
console.log(typeof n, n + 1, n.valueOf(), n.toString(), Object.prototype.toString.call(n));
function attempt(label, fn) {
  try {
    console.log(label, "->", fn());
  } catch (e) {
    console.log(label, "->", String(e));
  }
}
attempt("radix 1", () => (1).toString(1));
attempt("toFixed 101", () => (1).toFixed(101));
attempt("toPrecision 0", () => (1).toPrecision(0));
attempt("valueOf on string", () => Number.prototype.valueOf.call("1"));
console.log(Math.abs(-3), Math.ceil(1.2), Math.floor(-1.2), Math.round(2.5), Math.round(-2.5), Math.round(0.49999999999999994), Math.trunc(-4.7), Math.sign(-0));
console.log(Math.max(1, 5, 3), Math.min(1, 5, 3), Math.max(), Math.min(), Math.max(1, NaN), Math.max("7", 2));
console.log(Math.sqrt(16), Math.cbrt(-27), Math.hypot(3, 4), Math.hypot(), Math.pow(2, -2), Math.clz32(1), Math.imul(0xffffffff, 5), Math.fround(5.05));
console.log(Math.sin(0), Math.cos(0), Math.atan2(1, 1), Math.exp(0), Math.log(Math.E), Math.log2(8), Math.log10(1000), Math.expm1(0), Math.log1p(0));
console.log(Math.PI, Math.SQRT2, Math.LN2, Object.prototype.toString.call(Math), typeof Math.random());
let inRange = true;
for (let i = 0; i < 1000; i++) {
  const r = Math.random();
  if (!(r >= 0 && r < 1)) inRange = false;
}
console.log("random in [0, 1):", inRange);
