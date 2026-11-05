// The @fast entries of substring, at, repeat, padStart/padEnd and Number.isInteger (0230b)
// against qjs: primitive arguments take the entry; objects, symbols and every throwing case
// must give the builtin's own result and side effects.
function attempt(label, fn) {
  try {
    console.log(label, "->", JSON.stringify(fn()));
  } catch (e) {
    console.log(label, "->", String(e));
  }
}
const log = [];
const obj = (v) => ({ valueOf() { log.push("valueOf " + v); return v; }, toString() { log.push("toString " + v); return String(v); } });
const s = "hello world";
const two = "h\u00e9llo w\u0151rld \ud83d\ude00";

// substring
console.log(s.substring(1, 4), s.substring(4, 1), s.substring(-3, 2), s.substring(8), s.substring(NaN, Infinity), s.substring(3.7, 5.2), s.substring("2", "5"), s.substring(true, null));
console.log(JSON.stringify([s.substring(), s.substring(undefined, 3), s.substring(3, undefined), s.substring(99), s.substring(0, -1), two.substring(5, 15)]));
log.length = 0;
console.log(s.substring(obj(1), obj(4)), log.join(";"));
attempt("substring symbol", () => s.substring(Symbol("x")));
attempt("substring symbol end", () => s.substring(1, Symbol("x")));
attempt("substring null this", () => String.prototype.substring.call(null, 1));
console.log(String.prototype.substring.call(12345, 1, 3), String.prototype.substring.call(true, 1));

// at
console.log(JSON.stringify([s.at(0), s.at(-1), s.at(11), s.at(-12), s.at(1.5), s.at("2"), s.at(), s.at(NaN), s.at(-0), two.at(-1), two.at(-2), "".at(0)]));
log.length = 0;
console.log(s.at(obj(4)), log.join(";"));
attempt("at symbol", () => s.at(Symbol()));
attempt("at undefined this", () => String.prototype.at.call(undefined, 0));

// repeat
console.log(JSON.stringify(["ab".repeat(3), "ab".repeat(0), "".repeat(5), "x".repeat(2.9), "x".repeat("3"), "x".repeat(null), "x".repeat(NaN), "-".repeat(40).length, "".repeat(1e10)]));
log.length = 0;
console.log("ab".repeat(obj(2)), log.join(";"));
attempt("repeat -1", () => "x".repeat(-1));
attempt("repeat Infinity", () => "x".repeat(Infinity));
try { "x".repeat(2 ** 31); } catch (e) { console.log("repeat huge ->", e.name); }
attempt("repeat symbol", () => "x".repeat(Symbol()));
attempt("repeat null this", () => String.prototype.repeat.call(null, 2));

// padStart / padEnd
console.log(JSON.stringify(["42".padStart(6, "0"), "x".padEnd(4, "ab"), "abc".padStart(2), "abc".padEnd(6), "a".padStart(5, ""), "a".padEnd(3, undefined), "\u0151".padStart(3, "\u00e9"), "5".padStart(3, 0), "5".padEnd(4, null)]));
console.log(JSON.stringify(["x".padStart(NaN, "y"), "x".padStart(-3, "y"), "x".padEnd(3.9, "-"), "x".padEnd("4", "12"), "abc".padStart(10, "123"), two.padEnd(20, "\ud83d")]));
const long = "x".padStart(100001, "ab"), long2 = "ő".padEnd(9000), long3 = "y".padEnd(70000);
console.log(long.length, long.slice(0, 5), long.slice(-3), long2.length, long2.slice(0, 2), "a".padStart(8193, "é").slice(-3), long3.length, long3.slice(0, 2) + "|", "z".padStart(66000, "\u0151").slice(-2));
log.length = 0;
console.log("1".padStart(obj(3), obj("0")), "1".padEnd(obj(1), obj("0")), log.join(";"));
attempt("padEnd huge", () => "x".padEnd(2 ** 31));
attempt("padStart Infinity", () => "x".padStart(Infinity, "a"));
attempt("padStart Infinity empty filler", () => "x".padStart(Infinity, ""));
attempt("padStart symbol filler", () => "x".padStart(5, Symbol()));
attempt("padStart symbol filler unused", () => "xxxxx".padStart(5, Symbol()));
attempt("padEnd symbol length", () => "x".padEnd(Symbol()));
attempt("padEnd undefined this", () => String.prototype.padEnd.call(undefined, 3));

// Number.isInteger
console.log(Number.isInteger(5), Number.isInteger(5.5), Number.isInteger(-0), Number.isInteger(NaN), Number.isInteger(Infinity), Number.isInteger(-Infinity), Number.isInteger(2 ** 53), Number.isInteger(1e300), Number.isInteger(5e-324));
console.log(Number.isInteger("5"), Number.isInteger(null), Number.isInteger(), Number.isInteger(true), Number.isInteger(obj(1)), Number.isInteger(Symbol()), log.length);

// Patched builtins are called as patched.
const saved = String.prototype.substring;
String.prototype.substring = () => "patched";
console.log(s.substring(1, 2));
String.prototype.substring = saved;
console.log(s.substring(1, 2));

// String(v) and new String(v)
console.log(JSON.stringify([String(), String(42), String(-0), String(1e21), String(null), String(undefined), String(true), String("x"), String(12n)]));
console.log(String(Symbol("d")), String(obj(7)), typeof new String(5), new String(5).length, new String().length, JSON.stringify(new String("ab")));
attempt("new String symbol", () => new String(Symbol("e")));
class S extends String {}
console.log(new S("abc").length, new S("abc") instanceof S, Reflect.construct(String, ["q"], Object).constructor === Object);
console.log(String.call(null, 3), String.apply(undefined, [4, 5]));

// toFixed
console.log(JSON.stringify([(1.005).toFixed(2), (12.5).toFixed(), (0).toFixed(3), (-0).toFixed(1), (-1.5).toFixed(0), (1e21).toFixed(2), NaN.toFixed(2), (1.45).toFixed("1"), (123.456).toFixed(null), (5).toFixed(100).length]));
log.length = 0;
console.log((3.14159).toFixed(obj(2)), log.join(";"));
attempt("toFixed 101", () => (1).toFixed(101));
attempt("toFixed -1", () => (1).toFixed(-1));
attempt("toFixed symbol", () => (1).toFixed(Symbol()));
attempt("toFixed on string", () => Number.prototype.toFixed.call("1", 2));
console.log(Number.prototype.toFixed.call(Object(2.5), 1));
