// The @fast leaf paths (0085, D1850) against qjs: primitive arguments take the fast path,
// everything that could run user code or throw must give the builtin's own result.
function attempt(label, fn) {
  try {
    console.log(label, "->", fn());
  } catch (e) {
    console.log(label, "->", String(e));
  }
}
const log = [];
// Numbers as strings: the console's JSON form loses NaN, Infinity and -0.
const S = (...xs) => console.log(xs.map((x) => (Object.is(x, -0) ? "-0" : typeof x === "number" ? String(x) : x)).join(" "));
const obj = (v) => ({ valueOf() { log.push("valueOf " + v); return v; }, toString() { log.push("toString " + v); return String(v); } });
const s = "hello world";
const two = "h\u00e9llo w\u0151rld \ud83d\ude00";

// includes / indexOf / lastIndexOf / startsWith / endsWith
console.log(s.includes("wor"), s.includes("x"), s.includes(""), s.includes("o", 5), s.includes("o", 1e9), s.includes("h", -5), s.includes("o", NaN));
console.log(s.indexOf("o"), s.indexOf("o", 5), s.indexOf(""), s.indexOf("", 99), s.indexOf("d", 10), s.indexOf(undefined), "undefined!".indexOf(undefined));
console.log(s.lastIndexOf("o"), s.lastIndexOf("o", 5), s.lastIndexOf("o", -1), s.lastIndexOf(""), s.lastIndexOf("", 3), s.lastIndexOf("o", NaN), s.lastIndexOf("hello world!"));
console.log(s.startsWith("hel"), s.startsWith("wor", 6), s.startsWith("", 11), s.startsWith("x"), s.endsWith("ld"), s.endsWith("lo", 5), s.endsWith("h", 1), s.endsWith("", 0));
console.log(two.includes("\u0151"), two.indexOf("\ud83d"), two.lastIndexOf("l"), two.startsWith("h\u00e9"), two.endsWith("\ude00"), "abc".includes("\u0100"));
console.log((12345).toString().includes(3), "true".startsWith(true), "null".endsWith(null), "1,2".includes(","));
attempt("includes regexp", () => s.includes(/o/));
attempt("startsWith regexp", () => s.startsWith(/h/));
attempt("includes symbol", () => s.includes(Symbol("x")));
attempt("includes null this", () => String.prototype.includes.call(null, "a"));
attempt("includes undefined this", () => String.prototype.indexOf.call(undefined, "a"));
log.length = 0;
console.log(s.includes(obj("wor")), s.indexOf("o", obj(5)), s.startsWith(obj("he")), log.join(";"));
const re = /x/;
re[Symbol.match] = false;
attempt("includes non-regexp regexp", () => "/x/".includes(re));
console.log(String.prototype.includes.call(42, 2), String.prototype.indexOf.call(true, "u"));

// slice / substring / substr / charAt / charCodeAt / codePointAt / at
console.log(s.slice(-5), s.slice(2, -2), s.slice(5, 2), s.slice(), s.slice(NaN, 3), s.slice(-1e9, 1e9), s.slice(3.7, 5.2));
console.log(s.substring(4, 1), s.substring(-3, 2), s.substring(8), s.substring(NaN, Infinity), s.substr(-5, 3), s.substr(2), s.substr(3, -1));
S(s.charAt(1), s.charAt(-1), s.charAt(99), s.charAt(), s.charCodeAt(1), s.charCodeAt(99), s.codePointAt(0), s.codePointAt(99));
S(two.codePointAt(14), two.codePointAt(15), two.at(-1), two.at(-2), s.at(0), s.at(11), s.at(-12), s.at(1.5), s.at("2"));
log.length = 0;
console.log(s.slice(obj(1), obj(3)), s.charAt(obj(2)), log.join(";"));

// repeat / padStart / padEnd
console.log("ab".repeat(3), "ab".repeat(0), "".repeat(5), "x".repeat(2.9), "-".repeat(40).length);
attempt("repeat -1", () => "x".repeat(-1));
attempt("repeat Infinity", () => "x".repeat(Infinity));
console.log("42".padStart(6, "0"), "x".padEnd(4, "ab") + "|", "abc".padStart(2), "abc".padEnd(6) + "|", "a".padStart(5, ""), "a".padEnd(3, undefined) + "|", "\u0151".padStart(3, "\u00e9"));
console.log("5".padStart(3, 0), "5".padEnd(4, null), "x".padStart(NaN, "y"), "x".padStart(-3, "y"), "x".padEnd(3.9, "-"));
attempt("padEnd huge", () => "x".padEnd(2 ** 31));
log.length = 0;
console.log("1".padStart(obj(3), obj("0")), log.join(";"));

// case / trim
console.log(s.toUpperCase(), "\u00e9\u0151".toUpperCase(), "\u00df".toUpperCase(), "\u0130".toLowerCase().length, "MiXeD".toLowerCase(), "x".toLocaleUpperCase());
console.log("[" + " \t x \n".trim() + "]", "[" + "\u00a0x\ufeff".trim() + "]", "[" + " x ".trimStart() + "]", "[" + " x ".trimEnd() + "]", "[" + "".trim() + "]");
attempt("trim null", () => String.prototype.trim.call(null));
console.log(String.prototype.trim.call(123), String.prototype.toUpperCase.call(true));

// split
console.log("a,b,,c".split(","), "abc".split(""), "a b c".split(" ", 2), "abc".split(), "abc".split(undefined, 0), "".split(","), "".split(""));
console.log("a,b,c".split(",", -1), "a,b,c".split(",", 2 ** 32 + 1), "a1b1c".split(1), "anullb".split(null), "\u0151x\u0151".split("x"), "aaa".split("aa"));
log.length = 0;
console.log("a-b".split(obj("-")), "a-b-c".split("-", obj(2)), log.join(";"));
attempt("split symbol", () => "a".split(Symbol()));
const splitter = { [Symbol.split](str, lim) { return ["custom", str, lim]; } };
console.log("x".split(splitter, 3));

// replace / replaceAll
console.log(s.replace("o", "0"), s.replaceAll("o", "0"), s.replace("x", "y"), s.replace("", "_"), "aaa".replaceAll("", "-"), "aaa".replaceAll("aa", "b"));
console.log(s.replace("o", "$&$&"), s.replace("o", "[$`]"), s.replace("o", "$$"), s.replaceAll("l", "$'"), s.replace(1, 2), "1null".replace(null, "N"));
console.log(s.replace("o", () => "F"), s.replaceAll("o", (m, p) => p), s.replace("o", obj("Q")));
attempt("replaceAll non-global regexp", () => s.replaceAll(/o/, "x"));
console.log(s.replace(/o/g, "0"), "\u0151\u0151".replaceAll("\u0151", "o"), two.replace("w", "W"));

// String(), Number.prototype.toString / toFixed
console.log(String(12), String(-0), String(1e21), String(true), String(), String(null), String(undefined), String("s"), String(12n), String(Symbol("d")));
console.log(String(obj(7)), String([1, 2]), typeof new String("x"));
S((255).toString(16), (255).toString(), (-255).toString(2), (0.5).toString(10), (1e21).toString(36), (12).toString(undefined), (NaN).toString(16));
attempt("toString radix 1", () => (5).toString(1));
attempt("toString radix 37", () => (5).toString(37));
S((1.005).toFixed(2), (1e21).toFixed(2), (-1.5).toFixed(0), (0).toFixed(), (123.456).toFixed(10), (2.5).toFixed(0.9));
attempt("toFixed 101", () => (1).toFixed(101));
attempt("toFixed on string", () => Number.prototype.toFixed.call("1", 2));
console.log(Number.prototype.toString.call(new Number(7)), Number.prototype.toFixed.call(Object(1.25), 1));

// Math
S(Math.max(1, 5, 3), Math.max(), Math.min(), Math.min(0, -0), 1 / Math.max(-0, 0), 1 / Math.min(0, -0), Math.max(1, NaN), Math.min(NaN, 1));
S(Math.max("7", 3), Math.max(true, null), Math.min(undefined, 1), Math.max(1, 2, 3, 4, 5, 6, 7, 8, 9), Math.max(...[3, 9, 2]));
log.length = 0;
S(Math.max(obj(1), NaN, obj(2)), log.join(";"));
attempt("max symbol", () => Math.max(1, Symbol()));
attempt("max bigint", () => Math.max(1n));
S(Math.floor(2.7), Math.floor(-2.5), Math.ceil(-0.5), 1 / Math.ceil(-0.5), Math.round(-2.5), Math.round(2.5), Math.round(-0.4), Math.abs(-3), Math.abs("-2"));
S(Math.sqrt(16), Math.sqrt(-1), Math.trunc(-4.7), Math.sign(-3), Math.sign(-0), Math.exp(0), Math.log(Math.E), Math.log2(8), Math.log10(1000), Math.sin(0), Math.cos(0));
S(Math.floor(), Math.floor(null), Math.floor("x"), Math.floor(obj(3.5)));

// hasOwnProperty / Object.hasOwn / Array.isArray
const o = { a: 1, [Symbol.for("k")]: 2, 3: "three" };
console.log(o.hasOwnProperty("a"), o.hasOwnProperty("b"), o.hasOwnProperty(3), o.hasOwnProperty("3"), o.hasOwnProperty(Symbol.for("k")), o.hasOwnProperty("toString"));
console.log(Object.hasOwn(o, "a"), Object.hasOwn(o, 3), Object.hasOwn([1, 2], 1), Object.hasOwn([1, 2], "length"), Object.hasOwn("abc", 1), Object.hasOwn(function f() {}, "name"));
console.log([1].hasOwnProperty(0), [, 1].hasOwnProperty(0), new Uint8Array(2).hasOwnProperty(1), new Uint8Array(2).hasOwnProperty(5), "abc".hasOwnProperty("length"));
log.length = 0;
console.log(o.hasOwnProperty(obj("a")), log.join(";"));
const prox = new Proxy({}, { getOwnPropertyDescriptor(t, k) { log.push("gOPD " + String(k)); return undefined; } });
log.length = 0;
console.log(Object.hasOwn(prox, "z"), Object.prototype.hasOwnProperty.call(prox, "y"), log.join(";"));
attempt("hasOwn null", () => Object.hasOwn(null, "a"));
attempt("hasOwnProperty undefined this", () => Object.prototype.hasOwnProperty.call(undefined, "a"));
console.log(Array.isArray([]), Array.isArray("x"), Array.isArray({ length: 0 }), Array.isArray(new Proxy([], {})), Array.isArray(), Array.isArray(Array.prototype));
const { proxy: revoked, revoke } = Proxy.revocable([], {});
revoke();
attempt("isArray revoked", () => Array.isArray(revoked));

// Patched builtins: the call goes to whatever function the property holds.
const savedIncludes = String.prototype.includes;
String.prototype.includes = function (x) { return "patched " + x; };
console.log("abc".includes("b"));
String.prototype.includes = savedIncludes;
const savedMax = Math.max;
Math.max = () => "patched max";
S(Math.max(1, 2));
Math.max = savedMax;
console.log("abc".includes("b"), Math.max(1, 2));

// Through call/apply/bind, as getters' targets, and from builtins written in the
// builtin language (their ToString/ToIntegerOrInfinity helpers are fast too).
console.log(String.prototype.slice.call("abcdef", 1, 3), String.prototype.padStart.apply("7", [3, "0"]), Math.max.apply(null, [4, 8, 1]), Math.min.bind(null, 5)(2));
console.log(["b", "a", "c"].map(String.prototype.toUpperCase.call, String.prototype.toUpperCase), [" x", "y "].map((v) => v.trim()));
console.log("abc".localeCompare("abd"), "abcabc".lastIndexOf("c", 4), "x".concat(1, null, [2]), "\u00e9".normalize("NFD").length);
let n = 0;
const counter = { get v() { n++; return " a "; } };
console.log(counter.v.trim(), n);

// Number(), parseInt, parseFloat, isNaN, isFinite
S(Number("42"), Number(""), Number(" 0x1f "), Number(null), Number(undefined), Number(true), Number(12n), Number(), Number("1e3"), Number("abc"), Number(obj(4)));
S(parseInt("42px"), parseInt("ff", 16), parseInt("  -7"), parseInt("0x1A"), parseInt("12", 37), parseInt(15.99), parseInt(null, 36), parseInt("z", obj(36)));
S(parseFloat("3.14abc"), parseFloat(".5"), parseFloat("x"), parseFloat("-0"), parseFloat(obj("2.5")), Number.parseFloat("1e3"), Number.parseInt("08"));
S(isNaN("x"), isNaN("1"), isNaN(undefined), isFinite("1e400"), isFinite(null), isFinite(obj(1)));
attempt("Number symbol", () => Number(Symbol()));
attempt("parseInt symbol", () => parseInt(Symbol()));
console.log(typeof new Number(5), new Number("7") + 1);

// ArraySpeciesCreate (map/filter/slice/concat/flat results)
console.log([1, 2, 3].map((x) => x * 2), [1, 2, 3].filter((x) => x > 1), [1, 2, 3].slice(1), [1, 2].concat([3]), [[1], [2]].flat());
class MyArr extends Array {}
const myArr = MyArr.from([1, 2, 3]);
console.log(myArr.map((x) => x) instanceof MyArr, myArr.filter(Boolean).constructor.name, myArr.slice(0, 1) instanceof MyArr);
const noCtor = [1, 2];
noCtor.constructor = undefined;
console.log(noCtor.map((x) => x).length);
const ownSpecies = [1, 2];
ownSpecies.constructor = { [Symbol.species]: function (n) { console.log("species", n); return {}; } };
console.log(ownSpecies.map((x) => x));
const savedSpecies = Object.getOwnPropertyDescriptor(Array, Symbol.species);
let speciesGets = 0;
Object.defineProperty(Array, Symbol.species, { get() { speciesGets++; return undefined; }, configurable: true });
const viaGetter = [1].map((x) => x);
Object.defineProperty(Array, Symbol.species, savedSpecies);
console.log(viaGetter, speciesGets);
const savedCtor = Array.prototype.constructor;
Array.prototype.constructor = MyArr;
const viaCtor = [1].map((x) => x) instanceof MyArr;
Array.prototype.constructor = savedCtor;
console.log(viaCtor);
console.log([1].map((x) => x), Array.prototype.map.call({ length: 2, 0: "a", 1: "b" }, (x) => x + x), Array.prototype.map.call(new Proxy([1, 2], {}), (x) => x * 3));
const otherProto = [1];
Object.setPrototypeOf(otherProto, MyArr.prototype);
console.log(otherProto.map((x) => x) instanceof MyArr);
