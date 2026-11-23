"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 3);
globalThis.$dump = {};
async function main() {
  const x = new Thing(1, await later(2)).a.join();
  const y = new Thing(...[1], await later(2), 3).a.join();
  return x + " " + y;
}
$dump.p1 = $djs.resume(2, "later", [2]);
class Thing { constructor(...a) { this.a = a; } }
$dump.a2_0 = Thing;
$dump.Ka2 = async function () {
const $djs_C = $djs.C, $djs_Q = $djs.Q;
const x = "1,2";
const $djs_t0 = $dump.a2_0, $djs_t1 = [1];
const y = $djs_C($djs_t0, $djs_Q($djs_t1, (await $dump.p1), 3)).a.join();
return x + " " + y;
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: main, at: "await#2", segs: [126,83,1,180,121,1,183,123,2,185,125,5,190,130,2,194,136,19], scaf: [["k",0,130],["k",0,137],["site",105,163]], cmap: [["new.callee@87",0,"$djs_t0"],["new.args@96",0,"$djs_t1"]] } });
$dump.a2 = $dump.Ka2.call(undefined);
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
$djs.keep(Thing, []);
$djs.keep(Thing.prototype, []);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
