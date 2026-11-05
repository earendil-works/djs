"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 4);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(function* () {}).prototype;
async function main() {
  const g = counter(4);
  let r = g.next();
  while (!r.done) {
    const v = await later(r.value * 10);
    print("got", v);
    r = g.next(v);
  }
  print("total", r.value);
}
$dump.p1 = $djs.resume(3, "later", [10]);
var counter = function* counter(limit) {
  let total = 0;
  for (let i = 0; i < limit; i++) {
    const got = yield i;
    total += got;
  }
  return total;
};
$dump.n3 = { __proto__: $dump.b0 };
$dump.a2 = $djs.k(function* (limit, total) {
{ let $djs_f1 = true; for (let i = 1; $djs_f1 || (i < limit); i++) { if ($djs_f1) { $djs_f1 = false; 
{
const got = yield;
total += got;
}
 } else {
    const got = yield i;
    total += got;
  } } }
return total;
}, { f: counter, at: "yield#1", segs: [77,62,9,89,73,3,137,90,11,150,109,13,174,78,48,227,129,15], scaf: [["for",46,49],["site",96,143]] }, $dump.n3)(4, 0);
$dump.a2.next();
$dump.a4 = $djs.k(async function (g, r) {
{ let $djs_f1 = true; while ($djs_f1 || (!r.done)) { if ($djs_f1) { $djs_f1 = false; {
const v = await $dump.p1;
print("got", v);
r = g.next(v);
}
 } else {
    const v = await later(r.value * 10);
    print("got", v);
    r = g.next(v);
  } } }
print("total", r.value);
}, { f: main, segs: [65,77,7,117,98,10,137,133,17,154,154,14,179,86,86,270,175,26], scaf: [["while",70,46],["site",102,121]] })($dump.a2, { value: 1, done: false });
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
