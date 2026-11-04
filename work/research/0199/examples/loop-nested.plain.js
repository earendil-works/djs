"djs-plain 1";
globalThis.$dump = { R: Reflect.apply,
  G: Reflect.get,
  C: Reflect.construct,
  D: (o, k, v) => Object.defineProperty(o, k, { value: v, writable: true, enumerable: true, configurable: true }),
  A: Object.assign,
  L: (it, next) => { let first = true; const r = { next() { if (first) { first = false; return { value: undefined, done: false }; } return Reflect.apply(next ?? it.next, it, []); }, return(v) { const m = it.return; return m == null ? { done: true, value: v } : Reflect.apply(m, it, [v]); }, [Symbol.iterator]() { return r; } }; return r; },
  Q: (a, ...v) => { let n = a.length; for (const x of v) Object.defineProperty(a, n++, { value: x, writable: true, enumerable: true, configurable: true }); return a; },
  advance: (it, n) => { const next = it.next; for (let i = 0; i < n; i++) Reflect.apply(next, it, []); return it; } };
$dump.b0 = Object.getPrototypeOf([][Symbol.iterator]())["next"];
$dump.b1 = Object.getPrototypeOf(""[Symbol.iterator]())["next"];
async function main() {
  let i = 0;
  while (i < 2) {
    for (const a of ["x", "y"]) {
      for (const b of "pq") print(i, a, b, await later(a + b));
    }
    i++;
  }
}
$dump.p1 = later("yp");
$dump.i2 = $dump.advance(Array.prototype.values.call(["x", "y"]), 2);
$dump.i3 = $dump.advance(String.prototype[Symbol.iterator].call("pq"), 1);
$dump.a4_0 = print;
$dump.Ka4 = async function () {
const $djs_R = $dump.R, $djs_L = $dump.L;
let i = 0;
{ let $djs_f1 = true; while ($djs_f1 || (i < 2)) { if ($djs_f1) { $djs_f1 = false; {
{ let $djs_f3 = true; for (const $djs_v3 of $djs_L($dump.i2, $dump.b0)) { if ($djs_f3) { $djs_f3 = false; {
const a = "y";
{
{ let $djs_f5 = true; for (const $djs_v5 of $djs_L($dump.i3, $dump.b1)) { if ($djs_f5) { $djs_f5 = false; {
const b = "p";
const $djs_t0 = $dump.a4_0;
$djs_R($djs_t0, undefined, [0, "y", "p", (await $dump.p1)]);
} } else { const b = $djs_v5; print(i, a, b, await later(a + b)); } } }
}
} } else { const a = $djs_v3; {
      for (const b of "pq") print(i, a, b, await later(a + b));
    } } } }
i++;
} } else {
    for (const a of ["x", "y"]) {
      for (const b of "pq") print(i, a, b, await later(a + b));
    }
    i++;
  } } }
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: main, at: "await#1", segs: [114,46,5,512,106,1,525,117,35,586,70,1,599,87,71,677,163,4,691,53,118], scaf: [["while",39,95],["for-of",59,180],["for-of",95,305],["site",132,476]], cmap: [[null,"call.callee",434],[null,"call.this",434],["call.callee@117","call.arg",441],["call.this@117","call.arg",450],[null,"call.arg",461],["call.arg@123","array.element",462],["call.arg@126","array.element",465],["call.arg@129","array.element",470],["call.arg@132","array.element",475]] } });
$dump.a4 = $dump.Ka4.call(undefined);
try { $dump.a4 } finally { delete globalThis.$dump; }
