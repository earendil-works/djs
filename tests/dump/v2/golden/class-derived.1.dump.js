"djs-dump 2";
$dump.keep = (o, keys) => {
  for (const k of Reflect.ownKeys(o)) {
    if (!["length", "name", "prototype", "constructor"].includes(k) && !keys.includes(k)) delete o[k];
  }
  for (const k of keys) if (!Object.prototype.hasOwnProperty.call(o, k)) throw new TypeError("the class does not make " + String(k));
};
$dump.p1 = $dump.pending(1, "later", ["woof"]);
class Animal {
  constructor(name) { this.name = name; }
  speak() { return this.name + " speaks"; }
}
let Dog = ((Animal) => class Dog extends Animal {
  async speak() { const s = super.speak(); return s + " " + (await later("woof")); }
})(Animal);
const d = { __proto__: Dog.prototype, name: "rex" };
$dump.a2 = $dump.resume(Dog.prototype.speak, { at: "await#1", awaiting: $dump.p1, this: d, bindings: { s: "rex speaks" }, operands: [["binary.left", "rex speaks "]] });
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
$dump.keep(Animal, []);
$dump.keep(Animal.prototype, ["speak"]);
$dump.keep(Dog, []);
$dump.keep(Dog.prototype, ["speak"]);
