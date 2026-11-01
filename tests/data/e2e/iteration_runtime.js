// Array Iterators and spread over iterables (0030 part 4); for-of comes with 0029.
const r = (o) => Object.keys(o).map((k) => k + "=" + String(o[k])).join(",");
const it = [1, 2][Symbol.iterator]();
console.log(r(it.next()), r(it.next()), r(it.next()), r(it.next()));
console.log(it.next === [].keys().next, Object.getPrototypeOf(it) === Object.getPrototypeOf([].entries()));
const e = ["x", "y"].entries();
console.log(e.next().value, [...["p", "q"].keys()], [...[, 1].values()]);
try { it.next.call({}); } catch (err) { console.log(err); }

// Spread: arrays, strings, holes read through the prototype, custom iterators.
console.log([..."a\ud83d\ude00b"].length, [0, ...[1, 2], ...""].length);
const log = [];
Object.defineProperty(Array.prototype, 3, { get() { log.push("get3"); return "p3"; }, configurable: true });
const h = [0]; h.length = 5;
console.log([...h], log);
delete Array.prototype[3];
const a = [1, 2];
a[Symbol.iterator] = function () { let i = 3; return { next: () => ({ value: i, done: i-- <= 0 }) }; };
console.log([...a], [0, ...a, ...[7, 8]]);

// A replaced %ArrayIteratorPrototype%.next is used, and restoring it restores the fast path.
const AIP = Object.getPrototypeOf([][Symbol.iterator]());
const orig = AIP.next;
AIP.next = function () { const res = orig.call(this); if (!res.done) res.value *= 10; return res; };
console.log([...[1, 2, 3]]);
AIP.next = orig;
console.log([...[1, 2, 3]]);

// Calls and construction with spread; next, done and value are read in order.
function f(...xs) { return xs.length + ":" + xs.join("-"); }
console.log(f(...[1, 2], ...a, ..."hé"), Math.max(...[3, 9, 2]));
function P(...xs) { this.n = xs.length; }
console.log(new P(...[1, 2, 3]).n);
const steps = [];
const counter = {
  i: 0,
  [Symbol.iterator]() { steps.push("iter"); return this; },
  get next() { steps.push("next"); return () => ({ get done() { steps.push("done"); return counter.i > 2; }, get value() { steps.push("value"); return counter.i++; } }); },
};
console.log([...counter], steps.join(" "));

// Big spreads collect across GC safepoints.
const big = [];
for (let i = 0; i < 2000; i++) big.push({ i });
const gen = { [Symbol.iterator]() { let i = 0; return { next: () => (i < big.length ? { value: big[i++].i, done: false } : { done: true }) }; } };
const out = [...gen];
console.log(out.length, out[0], out[1999]);

// Errors.
[null, 5, {}, { [Symbol.iterator]: () => 1 }, { [Symbol.iterator]: () => ({ next: () => 1 }) }].forEach((bad) => {
  try { [...bad]; } catch (err) { console.log(err); }
});
try { f(...undefined); } catch (err) { console.log(err); }
