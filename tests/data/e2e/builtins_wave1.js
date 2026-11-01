// 0031 part 1: Error constructors, call/apply/bind (bound functions across suspension
// and GC), descriptors, push, String().
function probe(a, b) {
  return [this === undefined ? "u" : this, a, b, arguments.length];
}
const b = probe.bind("t", 1);
const bb = b.bind(null, 2);
console.log(b(5), bb(), bb(7), b.name, bb.name, b.length, bb.length);
console.log(probe.call(3, 4), probe.apply(null, [5, 6]), probe.apply(1));
function Pt(x, y) { this.x = x; this.y = y; }
const P = Pt.bind(null, 10);
const p = new P(20);
console.log(p.x, p.y, p instanceof Pt, p instanceof P);
const acc = [];
for (let i = 0; i < 50; i++) acc.push(probe.bind(i, i).call(null, i + 1)[0]);
console.log(acc.length, acc[0], acc[49]);
const e = new RangeError("r", { cause: "c" });
console.log(e, e.cause, e instanceof RangeError, e instanceof Error, e.constructor === RangeError);
console.log(TypeError("t"), Error(), String(new Error("s")), Error.prototype.toString.call({ name: "X" }));
try { null.f(); } catch (err) { console.log(err.constructor === TypeError, err); }
const o = {};
Object.defineProperty(o, "x", { get() { return 42; }, enumerable: true });
console.log(o.x, Object.getOwnPropertyNames(o), o.hasOwnProperty("x"), o.propertyIsEnumerable("x"));
const d = Object.getOwnPropertyDescriptor(o, "x");
console.log(typeof d.get, d.set, d.enumerable, d.configurable);
const arr = [];
console.log(arr.push(1, 2, 3), arr, Array.isArray(arr), Array(2).length, new Array(1, 2));
console.log(String(), String(1.5), Math.pow(2, 8));
try { new probe.call(); } catch (err) { console.log(err); }
console.log(probe.toString().length, Math.pow.toString());
