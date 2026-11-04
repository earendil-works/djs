"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
$dump.p1 = $djs.resume(1, "later", ["woof"]);
class Animal {
  constructor(name) { this.name = name; }
  speak() { return this.name + " speaks"; }
}
let Dog = ((Animal) => class Dog extends Animal {
  async speak() { const s = super.speak(); return s + " " + (await later("woof")); }
})(Animal);
const d = { __proto__: Dog.prototype, name: "rex" };
$dump.Ka2 = ({ __proto__: Animal.prototype, async K() {
const s = "rex speaks";
return ("rex speaks " + (await $dump.p1));
} }).K;
Object.defineProperty($dump.Ka2, "$djs", { value: { f: Dog.prototype.speak, scaf: [["site",59,61]], cmap: [["binary.left@48",1,"rex speaks "]] } });
$dump.a2 = $dump.Ka2.call(d);
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
$djs.keep(Animal, []);
$djs.keep(Animal.prototype, ["speak"]);
$djs.keep(Dog, []);
$djs.keep(Dog.prototype, ["speak"]);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
