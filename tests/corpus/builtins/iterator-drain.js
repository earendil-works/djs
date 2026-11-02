// Keys and values Array Iterators drained in one step by spread, Array.from,
// IterableToList and array destructuring rest (D2305), and where that must stop.
const show = (x) => JSON.stringify(x);
// Keys over holes: never read the element.
console.log(show([...Array(5).keys()]), show(Array.from(Array(3).keys())));
const holey = [1, , 3, , 5];
console.log(show([...holey.keys()]), show([...holey.values()]), show([...holey.entries()]));
// A hole reads the prototype chain, which can have a getter.
let reads = 0;
Object.defineProperty(Array.prototype, 3, { get() { reads++; return "proto"; }, configurable: true });
console.log(show([...holey.values()]), show(Array.from(holey.values())), reads);
console.log(show([...holey.keys()]), reads);
delete Array.prototype[3];
// A partly consumed iterator continues where it was; the drain finishes it once.
const it = [10, 20, 30, 40].values();
it.next();
console.log(show([...it]), show(it.next()), show([...it]));
const kt = Array(4).keys();
kt.next(); kt.next();
console.log(show(Array.from(kt)), show(kt.next()));
// Array destructuring rest and IterableToList (new Set, AggregateError).
const [a, b, ...rest] = Array(6).keys();
console.log(a, b, show(rest));
console.log(show([...new Set(Array(4).keys())]), new AggregateError([1, 2, 3].values()).errors.length);
// A replaced `next` is called every step.
const patched = [1, 2, 3].values();
let calls = 0;
const original = patched.next;
patched.next = function () { calls++; return original.call(this); };
console.log(show([...patched]), calls);
// Typed arrays.
const ta = new Int16Array([5, 6, 7]);
console.log(show([...ta.keys()]), show(Array.from(ta.values())), show([...ta.entries()]));
// Into an array that is not plain: a subclass, a frozen result, the iterated array itself.
class Sub extends Array {}
const s = Sub.from([1, 2, 3].keys());
console.log(s instanceof Sub, show(s), s.length);
function Frozen() { return Object.freeze([]); }
try { Array.from.call(Frozen, [1, 2].values()); } catch (e) { console.log(e.constructor.name); }
const self = [1, 2, 3];
function Same() { return self; }
console.log(show(Array.from.call(Same, self.values())), show(self));
const grow = [1, 2];
function Grow() { return grow; }
try { Array.from.call(Grow, grow.keys()); } catch (e) { console.log("grow", e.constructor.name); }
// Large keys ranges and spreading after existing elements.
const big = [...Array(100000).keys()];
console.log(big.length, big[99999], show([0, ...Array(3).keys(), 9]));
// Mapping goes element by element.
console.log(show(Array.from(Array(4).keys(), (k, i) => k * 10 + i)));
