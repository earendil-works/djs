// A derived class (extends a name) with a super call in an async method, instances.
class Animal {
  constructor(name) { this.name = name; }
  speak() { return this.name + " speaks"; }
}
class Dog extends Animal {
  async speak() { const s = super.speak(); return s + " " + (await later("woof")); }
}
const d = new Dog("rex");
d.speak().then(print);
