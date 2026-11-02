// Array.from(arrayLike, mapfn) runs the callback in one frame where Get observes nothing
// (D2306), and element by element where it might.
const show = (x) => JSON.stringify(x);
console.log(show(Array.from({ length: 5 }, (_, i) => i * 2)));
console.log(show(Array.from({ length: 3, 0: "a", 2: "c" }, (x, i) => x + i)));
// The callback gets two arguments, and thisArg.
console.log(show(Array.from({ length: 2 }, function (...a) { return [a.length, this.t]; }, { t: 7 })));
// A getter on the object or its prototype runs, in order.
const log = [];
const al = { length: 4, get 2() { log.push("get 2"); return "g"; } };
console.log(show(Array.from(al, (x, i) => { log.push("cb " + i); return x ?? i; })), show(log));
Object.defineProperty(Object.prototype, 1, { get() { return "proto"; }, configurable: true });
console.log(show(Array.from({ length: 3 }, (x) => x)));
delete Object.prototype[1];
// The callback changes the object it reads from.
const live = { length: 4 };
console.log(show(Array.from(live, (x, i) => { live[i + 1] = "set" + i; return x; })));
const live2 = { length: 3 };
console.log(show(Array.from(live2, (x, i) => {
  if (i === 0) Object.defineProperty(live2, 2, { get() { return "late getter"; } });
  return x;
})));
// A Proxy, a function and a typed array as array-likes.
const p = new Proxy({ length: 2 }, { get(t, k) { return typeof k === "symbol" ? undefined : k === "length" ? 2 : "p" + k; } });
console.log(show(Array.from(p, (x) => x)));
function f3(a, b, c) {}
console.log(show(Array.from(f3, (x, i) => i)));
// Errors from the callback, a non-callable, and a frozen result.
try { Array.from({ length: 3 }, (x, i) => { if (i === 1) throw new Error("at " + i); }); } catch (e) { console.log(e.message); }
try { Array.from({ length: 1 }, 5); } catch (e) { console.log(e.constructor.name); }
function Frozen(n) { return Object.freeze(new Array(n)); }
try { Array.from.call(Frozen, { length: 2 }, (x) => 1); } catch (e) { console.log(e.constructor.name); }
// A constructor with its own result; a subclass.
function Plain(n) { this.n = n; }
const r = Array.from.call(Plain, { length: 2 }, (x, i) => i + 1);
console.log(r instanceof Plain, r.n, r[0], r[1], r.length);
class Sub extends Array {}
const s = Sub.from({ length: 3 }, (x, i) => i);
console.log(s instanceof Sub, show(s));
// Strings map by code points.
console.log(show(Array.from("a😀b", (c, i) => c + i)));
// Generators and async callbacks still work as callbacks.
console.log(show(Array.from({ length: 3 }, (x, i) => typeof (async () => i)())));
console.log(Array.from({ length: 100000 }, (_, i) => i).reduce((a, b) => a + b, 0));
