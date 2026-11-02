// The callback builtins on empty and one- or two-element arrays (D2308): an empty array
// never enters the dense loop, a short one enters it from the tier-0 loop's exit.
const show = (x) => JSON.stringify(x);
const calls = [];
const f = (x, i, a) => { calls.push([x, i, a.length]); return x; };
for (const a of [[], [7], [7, 8]]) {
  calls.length = 0;
  const r = [a.map(f), a.filter(f), a.some(f), a.every(f), a.find(f), a.findIndex(f),
    a.reduce((s, x) => s + x, 0), a.forEach(f), a.findLast(f), a.reduceRight((s, x) => s + x, 1)];
  console.log(a.length, show(r), calls.length);
}
// The checks still run on an empty array.
for (const m of ["map", "filter", "forEach", "some", "every", "find", "findIndex"]) {
  try { [][m](5); } catch (e) { console.log(m, e.constructor.name); }
}
try { [].reduce((s, x) => s); } catch (e) { console.log("reduce", e.constructor.name, e.message); }
// Species on an empty array.
class Sub extends Array {}
const s = Sub.from([]);
console.log(s.map(f) instanceof Sub, s.filter(f) instanceof Sub, Sub.from([1]).map((x) => x * 2) instanceof Sub);
// Throws from the first element, and from a nested call.
try { [1].map(() => { throw new Error("first"); }); } catch (e) { console.log(e.message); }
const tree = (d) => (d === 0 ? { v: 1, c: [] } : { v: d, c: [tree(d - 1), tree(d - 1)] });
const sum = (n) => n.v + n.c.map(sum).reduce((a, b) => a + b, 0);
console.log(sum(tree(8)));
// A generator and an async function as callbacks.
console.log(show([1, 2].map(function* (x) { yield x; }).map((g) => g.next().value)));
Promise.all([1].map(async (x) => x + 1)).then((r) => console.log("async", show(r)));
// Array.from of a short array-like.
console.log(show(Array.from({ length: 0 }, f)), show(Array.from({ length: 1 }, (x, i) => i)));
