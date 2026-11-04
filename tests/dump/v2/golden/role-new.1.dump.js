"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
async function main() {
  const x = new Thing(1, await later(2)).a.join();
  const y = new Thing(...[1], await later(2), 3).a.join();
  return x + " " + y;
}
$dump.p1 = $djs.resume(1, "later", [2]);
class Thing { constructor(...a) { this.a = a; } }
$dump.a2_0 = Thing;
$dump.Ka2 = async function () {
const $djs_C = $djs.C;

const $djs_t0 = $dump.a2_0;
const x = $djs_C($djs_t0, [1, (await $dump.p1)]).a.join();
const y = new Thing(...[1], await later(2), 3).a.join();
return x + " " + y;
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: main, segs: [78,32,1,120,64,2,122,66,5,127,71,2,131,77,56,188,136,19], scaf: [["k",0,82],["k",0,89],["tmp",0,99,"new.arg@46","array.element"],["site",49,103],["tmp",0,102,"new.arg@49","array.element"],["k",0,98]], cmap: [["new.callee@36",0,"$djs_t0"],["new.arg@46",1,1]] } });
$dump.a2 = $dump.Ka2.call(undefined);
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
$djs.keep(Thing, []);
$djs.keep(Thing.prototype, []);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
