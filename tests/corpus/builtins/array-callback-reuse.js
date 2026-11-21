// The callback frame reused element after element (0272): from the tier-0 loop too. Deep
// calls inside the callback (the stack may grow and move), a callback that throws midway,
// arrays that change length or get holes during the loop, a store that the fast path
// cannot make (a frozen species result), and results that stop the search.
"use strict";
function depth(n) { return n === 0 ? 0 : 1 + depth(n - 1); }
const a = [];
for (let i = 0; i < 40; i++) a.push(i);
console.log(a.map((x) => depth(x * 20)).reduce((s, x) => s + x, 0));
console.log(a.filter((x) => depth(x * 10) % 3 === 0).length);
let seen = 0;
try {
  a.forEach((x) => { seen++; if (x === 17) throw new Error("at " + x); });
} catch (e) { console.log(e.message, seen); }
const b = a.slice();
console.log(b.map((x, i, arr) => { if (i === 5) arr.length = 10; return x; }).length);
const c = a.slice(0, 8);
console.log(c.map((x, i, arr) => { if (i === 2) delete arr[4]; return x * 2; }).join());
console.log(a.some((x) => x === 33), a.every((x) => x < 33), a.find((x) => x > 20), a.findIndex((x) => x > 30));
class Frozen extends Array {
  static get [Symbol.species]() { return function (n) { return Object.freeze([]); }; }
}
try { Frozen.from([1, 2, 3]).map((x) => x); } catch (e) { console.log(e.constructor.name); }
const sums = [];
for (let r = 0; r < 3; r++) sums.push([1, 2, 3, 4].map((x) => x * r).filter((x) => x > 2).reduce((s, x) => s + x, 0));
console.log(sums.join());
