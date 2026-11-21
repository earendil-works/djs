// The comparator sort runs a plain closure's comparisons in one reused frame while their
// results are Numbers (0262): mixed result types switch between that loop and the
// builtin's ToNumber, other callables take the plain call, and the comparator can sort,
// throw, see its own arguments and change the array while it runs.
function attempt(label, fn) {
  try {
    console.log(label, "->", JSON.stringify(fn()));
  } catch (e) {
    console.log(label, "->", String(e));
  }
}
const base = Array.from({ length: 40 }, (_, i) => (i * 37) % 41);

// Number results, stability by a key, many runs.
const rows = base.map((v, i) => ({ k: v % 5, i }));
console.log(rows.slice().sort((a, b) => a.k - b.k).map((r) => r.k + ":" + r.i).join());
console.log(base.slice().sort((a, b) => b - a).join());

// Results of every type, mixed (consistent, so the order is defined): strings, booleans,
// null, undefined, objects with valueOf.
let n = 0;
const mixed = base.slice().sort((a, b) => {
  const d = a - b;
  n++;
  switch ((a + b) % 6) {
    case 0: return d;
    case 1: return String(d);
    case 2: return { valueOf: () => d };
    case 3: return d > 0 ? true : d < 0 ? -1 : false;
    case 4: return d < 0 ? -0.5 : d > 0 ? "1" : null;
    default: return d === 0 ? undefined : d * 1e300;
  }
});
console.log(mixed.join(), n > 0);

// Other callables: bound, native, class-free arrow with a default, a generator function.
const cmp = function (a, b) { return this.dir * (a - b); };
console.log(base.slice(0, 10).sort(cmp.bind({ dir: -1 })).join());
console.log([3, 1, 2].sort(Math.min).join(), [3, 1, 2].sort(Number).join());
console.log([5, 2, 8].sort((a, b = 0, ...rest) => a - b + rest.length).join());
attempt("generator comparator", () => [3, 1, 2].sort(function* () { yield 1; }));

// The comparator's own frame: its arguments object, locals, closures made per call.
const fns = [];
console.log([4, 2, 3, 1].sort(function (a, b) {
  let local;
  fns.push(() => local);
  local = arguments.length + ":" + a + b;
  return arguments[0] - arguments[1];
}).join(), fns.length > 0, fns[0]().split(":")[0]);

// Sorts inside the comparator, a throw after some comparisons, a throwing valueOf.
console.log([3, 1, 2].sort((a, b) => [b, a].sort((x, y) => x - y)[0] === a ? -1 : 1).join());
let calls = 0;
attempt("throw midway", () => base.slice().sort((a, b) => { if (++calls === 30) throw new Error("cmp " + calls); return a - b; }));
attempt("valueOf throws", () => [2, 1, 3].sort(() => ({ valueOf() { throw new TypeError("v"); } })));
attempt("symbol result", () => [2, 1, 3].sort((a, b) => (a === 3 ? Symbol() : a - b)));
attempt("bigint result", () => [2, 1].sort(() => 1n));

// The array changed by the comparator (the order is then implementation-defined).
const live = base.slice(0, 8);
console.log(live.sort((a, b) => { live.length = 3; live.push(99); return a - b; }).length);
const holes = [5, , 3, undefined, 1, , 4];
console.log(holes.sort((a, b) => a - b).map(String).join(), holes.length, 1 in holes, 6 in holes);
console.log(Array.prototype.sort.call({ length: 3, 0: "c", 1: "a", 2: "b" }, (a, b) => (a < b ? -1 : a > b ? 1 : 0)));
console.log([1, 2, 3].toSorted((a, b) => b - a).join(), [].sort((a, b) => a - b).length, [7].sort(() => 1).join());
