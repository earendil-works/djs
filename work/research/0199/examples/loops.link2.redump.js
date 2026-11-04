"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(3, "later", ["xb"]);
var letters = function* letters() {
  yield "a";
  yield "b";
};
async function main() {
  const m = new Map([["x", 1], ["y", 2]]);
  for (const [k, v] of m) {
    for (const l of letters()) {
      print(k, v, l, await later(k + l));
    }
  }
  print("end");
}
$dump.n2 = async function () {
const $djs_R = $dump.R, $djs_L = $dump.L;
const m = $dump.n2;
{ let $djs_f1 = true; for (const $djs_v1 of $djs_L($dump.i3, $dump.b0)) { if ($djs_f1) { $djs_f1 = false; {
const k = "x";
const v = 1;
{
{ let $djs_f3 = true; for (const $djs_v3 of $djs_L($dump.a4, $dump.b1)) { if ($djs_f3) { $djs_f3 = false; {
const l = "a";
{
const $djs_t0 = $dump.a5_0;
$djs_R($djs_t0, undefined, ["x", 1, "a", (await $dump.p1)]);
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
$dump.n3 = (it, next) => { let first = true; const r = { next() { if (first) { first = false; return { value: undefined, done: false }; } return Reflect.apply(next ?? it.next, it, []); }, return(v) { const m = it.return; return m == null ? { done: true, value: v } : Reflect.apply(m, it, [v]); }, [Symbol.iterator]() { return r; } }; return r; };
$dump.n4 = new Map([["x", 1], ["y", 2]]);
$dump.i7 = $dump.advance(Map.prototype.entries.call($dump.n4), 1);
$dump.s6 = (() => {
  let first = false;
  let next = $dump.builtin("%MapIteratorPrototype%.next");
  let it = $dump.i7;
  return { $f8: $dump.fn({ next() { if (first) { first = false; return { value: undefined, done: false }; } return Reflect.apply(next ?? it.next, it, []); } }), $f12: $dump.fn({ return(v) { const m = it.return; return m == null ? { done: true, value: v } : Reflect.apply(m, it, [v]); } }) };
})();
$dump.s8 = (() => {
  let r;
  return { $f13: $dump.fn({ [Symbol.iterator]() { return r; } }), $set(v) { ({ r } = v); } };
})();
$dump.n5 = { next: $dump.s6.$f8, "return": $dump.s6.$f12, [Symbol.iterator]: $dump.s8.$f13 };
$dump.n11 = function* () {
(yield);
yield "b";
};
$dump.a12 = $dump.resume($dump.n11, { at: "yield#2" });
$dump.s10 = (() => {
  let first = false;
  let next = $dump.builtin("%GeneratorPrototype%.next");
  let it = $dump.a12;
  return { $f10: $dump.fn({ next() { if (first) { first = false; return { value: undefined, done: false }; } return Reflect.apply(next ?? it.next, it, []); } }), $f17: $dump.fn({ return(v) { const m = it.return; return m == null ? { done: true, value: v } : Reflect.apply(m, it, [v]); } }) };
})();
$dump.s13 = (() => {
  let r;
  return { $f18: $dump.fn({ [Symbol.iterator]() { return r; } }), $set(v) { ({ r } = v); } };
})();
$dump.n9 = { next: $dump.s10.$f10, "return": $dump.s10.$f17, [Symbol.iterator]: $dump.s13.$f18 };
$dump.a14 = $dump.resume($dump.n2, { at: "await#2", awaiting: $dump.p1, slots: { $djs_R: Reflect.apply, $djs_L: $dump.n3, m: $dump.n4, $djs_f1: false, $djs_v1: undefined, "for-of.iterator@104": $dump.n5, "for-of.next@104": $dump.s6.$f8, k: "x", v: 1, $djs_f3: false, $djs_v3: "b", "for-of.iterator@242": $dump.n9, "for-of.next@242": $dump.s10.$f10, l: "b", "call.callee@474": print, "call.this@474": undefined, "call.arg@480": "x", "call.arg@483": 1, "call.arg@486": "b" } });
$dump.handle("run", $dump.a14);
Object.defineProperty($dump.n2, "$djs", { value: { f: main, at: "await#1", segs: [453, 110, 1, 466, 126, 49, 541, 80, 6, 559, 93, 86, 652, 182, 13], scaf: [["for-of", 69, 104], ["for-of", 99, 242], ["site", 149, 415]], cmap: [[null, "call.callee", 373], [null, "call.this", 373], ["call.callee@134", "call.arg", 380], ["call.this@134", "call.arg", 389], [null, "call.arg", 400], ["call.arg@140", "array.element", 401], ["call.arg@143", "array.element", 406], ["call.arg@146", "array.element", 409], ["call.arg@149", "array.element", 414]] }, writable: false, enumerable: false, configurable: false });
Object.defineProperty($dump.n3, "name", { value: "L" });
$dump.s8.$set({ r: $dump.n5 });
Object.defineProperty($dump.n11, "$djs", { value: { f: letters, at: "yield#1", segs: [24, 37, 10], scaf: [["site", 24, 16]], temps: [], cmap: [] }, writable: false, enumerable: false, configurable: false });
$dump.s13.$set({ r: $dump.n9 });
