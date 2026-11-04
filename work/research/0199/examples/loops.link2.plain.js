"djs-plain 1";
globalThis.$dump = { R: Reflect.apply,
  G: Reflect.get,
  C: Reflect.construct,
  D: (o, k, v) => Object.defineProperty(o, k, { value: v, writable: true, enumerable: true, configurable: true }),
  A: Object.assign,
  L: (it, next) => { let first = true; const r = { next() { if (first) { first = false; return { value: undefined, done: false }; } return Reflect.apply(next ?? it.next, it, []); }, return(v) { const m = it.return; return m == null ? { done: true, value: v } : Reflect.apply(m, it, [v]); }, [Symbol.iterator]() { return r; } }; return r; },
  Q: (a, ...v) => { let n = a.length; for (const x of v) Object.defineProperty(a, n++, { value: x, writable: true, enumerable: true, configurable: true }); return a; },
  advance: (it, n) => { const next = it.next; for (let i = 0; i < n; i++) Reflect.apply(next, it, []); return it; } };
$dump.b0 = Object.getPrototypeOf(new Map()[Symbol.iterator]())["next"];
$dump.b1 = Object.getPrototypeOf(function* () {}).prototype["next"];
async function main() {
  const m = new Map([["x", 1], ["y", 2]]);
  for (const [k, v] of m) {
    for (const l of letters()) {
      print(k, v, l, await later(k + l));
    }
  }
  print("end");
}
$dump.p1 = later("xb");
var letters = function* letters() {
  yield "a";
  yield "b";
};
$dump.n4 = new Map([["x", 1], ["y", 2]]);
$dump.i7 = $dump.advance(Map.prototype.entries.call($dump.n4), 1);
$dump.Ka12 = function* () {
(yield);
};
Object.defineProperty($dump.Ka12, "$djs", { value: { f: letters, at: "yield#2", segs: [], scaf: [["site",37,16]], temps: [], cmap: [] } });
$dump.Ka12.prototype = letters.prototype;
$dump.a12 = $dump.Ka12.call(undefined);
$dump.a12.next();
$dump.a14_0 = print;
$dump.Ka14 = async function () {
const $djs_R = $dump.R, $djs_L = $dump.L;
const m = $dump.n4;
{ let $djs_f1 = true; for (const $djs_v1 of $djs_L($dump.i7, $dump.b0)) { if ($djs_f1) { $djs_f1 = false; {
const k = "x";
const v = 1;
{
{ let $djs_f3 = true; for (const $djs_v3 of $djs_L($dump.a12, $dump.b1)) { if ($djs_f3) { $djs_f3 = false; {
const l = "b";
{
const $djs_t0 = $dump.a14_0;
$djs_R($djs_t0, undefined, ["x", 1, "b", (await $dump.p1)]);
}
} } else { const l = $djs_v3; {
      print(k, v, l, await later(k + l));
    } } } }
}
} } else { const [k, v] = $djs_v1; {
    for (const l of letters()) {
      print(k, v, l, await later(k + l));
    }
  } } } }
print("end");
};
Object.defineProperty($dump.Ka14, "$djs", { value: { f: main, at: "await#1", segs: [455,110,1,468,126,49,543,80,6,561,93,86,654,182,13], scaf: [["for-of",69,104],["for-of",99,242],["site",149,417]], cmap: [[null,"call.callee",375],[null,"call.this",375],["call.callee@134","call.arg",382],["call.this@134","call.arg",391],[null,"call.arg",402],["call.arg@140","array.element",403],["call.arg@143","array.element",408],["call.arg@146","array.element",411],["call.arg@149","array.element",416]] } });
$dump.a14 = $dump.Ka14.call(undefined);
try { $dump.a14 } finally { delete globalThis.$dump; }
