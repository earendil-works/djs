// reduce and reduceRight take initialValue by argument count, not by a rest array (0162):
// an explicit undefined is an initial value, extra arguments are ignored, and the dense
// loop carries the accumulator in its state (the callback's `this` stays undefined).
"use strict";
const add = (s, x) => s + x;
console.log([1, 2].reduce(add, undefined), [1, 2].reduce(add), [1, 2].reduce(add, 10, 20, 30));
console.log([1, 2].reduceRight(add, undefined), [1, 2].reduceRight(add), [].reduceRight(add, "r", 1));
console.log([].reduce(add, undefined), [undefined].reduce(add));
try { [].reduceRight(add); } catch (e) { console.log(e.constructor.name, e.message); }
const thisSeen = [];
[1, 2, 3].reduce(function (s, x) { thisSeen.push(this); return s + x; }, 0);
[1, 2, 3].reduce(function (s, x) { thisSeen.push(this); return s + x; });
console.log(thisSeen.map((t) => typeof t).join());
// The accumulator object passes through unchanged.
const acc = { n: 0 };
const r = [1, 2, 3].reduce((a, x) => { a.n += x; return a; }, acc);
console.log(r === acc, r.n);
// Nested reduces inside the callback do not disturb the outer accumulator.
const m = [[1, 2], [3], [], [4, 5, 6]];
console.log(m.reduce((s, row) => s + row.reduce(add, 0) * 10, 1));
// The array shrinks and grows during the loop, holes past the dense part.
const a = [1, 2, 3, 4, 5];
console.log(a.reduce((s, x, i, arr) => { if (i === 1) arr.length = 3; return s + x; }, 0));
const b = [1, 2, , 4];
console.log(b.reduce((s, x, i) => s + ":" + i + "=" + x, "h"));
const c = [1, 2];
console.log(c.reduce((s, x, i, arr) => { if (arr.length < 5) arr.push(x * 10); return s + x; }, 0));
// Array-likes and the callback's arguments.
console.log(Array.prototype.reduce.call({ length: 3, 0: "a", 2: "c" }, (s, x, i, o) => s + x + i + typeof o, ">"));
console.log(Array.prototype.reduce.length, Array.prototype.reduceRight.length);
// Throwing from the callback in the dense loop.
try { [1, 2, 3].reduce((s, x) => { if (x === 2) throw new Error("at " + s); return s + x; }, 0); } catch (e) { console.log(e.message); }
