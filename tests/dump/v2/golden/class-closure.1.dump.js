"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
function make(prefix) {
  return class Tagged {
    constructor(v) { this.v = v; }
    async tag() { return prefix + (await later(this.v)); }
  };
}
$dump.p1 = $djs.resume(1, "later", [4]);
$dump.s2 = (() => {
  let $c1 = class Tagged {
    constructor(v) { this.v = v; }
    async tag() { return prefix + (await later(this.v)); }
  };
  let prefix = "#";
  return { Tagged: $c1, $Ka3: async function () {

return ("#" + (await $dump.p1));
} };
})();
const T = $dump.s2.Tagged;
const t = { __proto__: $dump.s2.Tagged.prototype, v: 4 };
Object.defineProperty($dump.s2.$Ka3, "$djs", { value: { f: $dump.s2.Tagged.prototype.tag, scaf: [["site",31,36]], cmap: [["binary.left@21",1,"#"]] } });
$dump.a3 = $dump.s2.$Ka3.call(t);
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
$djs.keep($dump.s2.Tagged, []);
$djs.keep($dump.s2.Tagged.prototype, ["tag"]);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
