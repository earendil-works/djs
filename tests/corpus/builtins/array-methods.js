// Array.prototype in full (0034), checked against qjs.
// difftest: no-stress 7 s suspended, 23 s under GC stress (debug, 0053)
function attempt(label, fn) {
  try {
    console.log(label, "->", fn());
  } catch (e) {
    console.log(label, "->", String(e));
  }
}
const base = () => [5, 1, 4, 2, 3];
console.log(base().at(-1), base().at(9), base().lastIndexOf(4), base().indexOf(9), base().includes(NaN), [NaN].includes(NaN));
console.log(base().find((v) => v > 3), base().findIndex((v) => v > 3), base().findLast((v) => v > 3), base().findLastIndex((v) => v > 9));
console.log(base().every((v) => v > 0), base().some((v) => v > 4), base().filter((v) => v % 2), base().map((v) => v * 10));
console.log(base().reduce((a, b) => a + b), base().reduceRight((a, b) => a + "," + b, "start"));
console.log(base().concat([6, [7]], 8), base().slice(1, -1), base().slice(-2), base().join("-"), String(base()));
const sp = base();
console.log(sp.splice(1, 2, "a", "b", "c"), sp, sp.splice(-1), sp, sp.splice(1), sp);
const sh = [1, 2, 3, 4];
console.log(sh.shift(), sh, sh.unshift(0, 0.5), sh, sh.pop(), sh, sh.push(9, 10), sh);
const holes = [1, , 3, , 5];
console.log(holes.shift(), holes.length, 1 in holes, holes.unshift(7), 2 in holes, holes.splice(1, 1), holes.length);
console.log([1, 2, 3, 4, 5].copyWithin(0, 3), [1, 2, 3, 4, 5].copyWithin(1, 0, 3), new Array(4).fill(0, 1, 3), [3, 1, 2].reverse());
console.log([1, [2, [3, [4]]]].flat(), [1, [2, [3, [4]]]].flat(Infinity), [1, 2].flatMap((v) => [v, v * 2]));
console.log([10, 9, 1, 100, 25].sort(), [10, 9, 1, 100, 25].sort((a, b) => a - b), ["b", undefined, "a", , "c"].sort());
const stable = [];
for (let i = 0; i < 30; i++) stable.push({ k: i % 3, i });
console.log(stable.sort((a, b) => a.k - b.k).map((e) => e.i).join(","));
console.log([3, 1, 2].toSorted(), [1, 2, 3].toReversed(), [1, 2, 3].toSpliced(1, 1, "x", "y"), [1, 2, 3].with(-1, 0));
console.log(Array.of(7, 8), Array.from({ length: 3, 0: "a" }), Array.from("héllo"), Array.from([1, 2], (v) => v * 3));
console.log(Array.prototype[Symbol.unscopables].flat, Object.keys(Array.prototype[Symbol.unscopables]).length, Array[Symbol.species] === Array);
console.log([1, null, undefined, [2, 3], { toString() { return "o"; } }].toString(), [1.5, 2].toLocaleString());
attempt("reduce empty", () => [].reduce((a, b) => a));
attempt("sort bad comparator", () => [1, 2].sort(5));
attempt("with out of range", () => [1].with(3, 0));
attempt("pop frozen", () => Object.freeze([1]).pop());
attempt("shift frozen", () => Object.freeze([1, 2]).shift());
attempt("unshift too long", () => Array.prototype.unshift.call({ length: 2 ** 53 - 1 }, 1));
attempt("splice on array-like", () => {
  const o = { length: 3, 0: "a", 1: "b", 2: "c" };
  return [Array.prototype.splice.call(o, 1, 1, "x", "y"), o.length, o[0], o[1], o[2], o[3]];
});
attempt("shift on array-like", () => {
  const o = { length: 2, 0: "a", 1: "b" };
  return [Array.prototype.shift.call(o), o.length, o[0], 1 in o];
});
attempt("species", () => {
  const a = [1, 2, 3];
  a.constructor = { [Symbol.species]: function (n) { this.made = n; } };
  const r = a.filter(() => true);
  return [r.made, r[0], r.length];
});
const big = [];
for (let i = 0; i < 2000; i++) big.push(i);
let total = 0;
while (big.length) total += big.shift();
console.log(total);
