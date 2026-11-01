// 0031 part 2: Object statics, wrappers, Boolean, Symbol, Number, Math, globals.
const o = { b: 2, a: 1, 3: "x" };
console.log(Object.keys(o), Object.values(o), Object.entries(o));
console.log(Object.assign({}, o, { c: 3 }), Object.isFrozen(Object.freeze({})), Object.getPrototypeOf([]) === Array.prototype);
const c = Object.create({ up: 1 }, { own: { value: 2, enumerable: true } });
console.log(c.up, c.own, Object.keys(c), Object.prototype.toString.call(c), String(c));
console.log((255).toString(16), (3.14159).toFixed(2), (1234.5).toExponential(1), (0.000123).toPrecision(2));
console.log(new Number(4) * 2, typeof new Boolean(false), new Boolean(false).valueOf(), Object("ab").length, Object("ab")[1]);
console.log(parseInt("ff", 16), parseFloat("2.5x"), isNaN("q"), isFinite("1"), Number("0b101"), Number.isInteger(3));
const s = Symbol("tag");
console.log(s.toString(), s.description, Symbol.for("a") === Symbol.for("a"), Symbol.keyFor(Symbol.for("a")), Symbol.iterator.description);
console.log(Math.max(3, 9, 1), Math.min(3, 9, 1), Math.round(-0.5), Math.hypot(5, 12), Math.trunc(9.9), Math.sign(-2), Math.abs(-1.5));
const xs = [];
for (let i = 0; i < 20; i++) xs.push(Math.floor(Math.random() * 10));
let ok = true;
for (let i = 0; i < xs.length; i++) if (xs[i] < 0 || xs[i] > 9 || xs[i] !== Math.trunc(xs[i])) ok = false;
console.log("random ok", ok);
let sum = 0;
for (let i = 0; i < 30; i++) sum += Object.keys({ a: i, b: i }).length + Number(String(i)) % 3;
console.log(sum);
try { Symbol() + 1; } catch (e) { console.log(e); }
try { Object.defineProperty(Object.freeze({ k: 1 }), "k", { value: 2 }); } catch (e) { console.log(e); }
