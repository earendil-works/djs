"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
$dump.p1 = $djs.resume(1, "later", ["m"]);
class Base { m(x) { return "base " + x; } }
let Child = ((Base) => class Child extends Base {
  async m(x) { return super[await later("m")](x); }
})(Base);
$dump.n2 = { __proto__: Child.prototype };
$dump.a3_0 = Base.prototype;
$dump.Ka3 = ({ __proto__: Base.prototype, async K() {
var x = 7;
return super[(await $dump.p1)](x);
} }).K;
Object.defineProperty($dump.Ka3, "$djs", { value: { f: Child.prototype.m, segs: [30,20,6,53,43,3], scaf: [["site",26,37]] } });
$dump.a3 = $dump.Ka3.call($dump.n2);
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
$djs.keep(Base, []);
$djs.keep(Base.prototype, ["m"]);
$djs.keep(Child, []);
$djs.keep(Child.prototype, ["m"]);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
