"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", ["woof"]);
class Animal {
  constructor(name) { this.name = name; }
  speak() { return this.name + " speaks"; }
}
let Dog = ((Animal) => class Dog extends Animal {
  async speak() { const s = super.speak(); return s + " " + (await later("woof")); }
})(Animal);
const d = { __proto__: Dog.prototype, name: "rex" };
$dump.a2 = $dump.resume(Dog.prototype.speak, { at: "await#1", awaiting: $dump.p1, this: d, slots: { s: "rex speaks", "binary.left@48": "rex speaks " } });
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
$dump.keep(Animal, []);
$dump.keep(Animal.prototype, ["speak"]);
$dump.keep(Dog, []);
$dump.keep(Dog.prototype, ["speak"]);
