// Neighbours of pjz's subclass cases (codemode/plain-fuzz/subclass-before-declaration.js,
// super-in-async-method-redump.js, 0215): a subclass's async method that awaits
// `super.fetch(...)`, then tools, then reads `super` again (its continuation uses `super`
// itself, so it has a home of its own), re-dumped at each await; and a subclass a closure
// names before its declaration.
class C {
  constructor(v) { this.value = v; }
  async fetch(x) { return this.value + (await tools.echo(x)); }
  get(k) { return k * 10 + this.value; }
}
class D extends C {
  constructor(v) { super(v * 2); this.extra = v; }
  async fetch(x) {
    const a = await super.fetch(x);
    const b = await tools.echo(this.extra);
    const c = await tools.echo(b + 1);
    return a + b + c + super.get(3);
  }
}
console.log(await new D(1).fetch(2));
const make = () => E.make(1);
class B { static make(v) { return new this(v); } constructor(v) { this.v = v; } }
class E extends B { async twice() { return (await tools.echo(this.v)) * 2; } }
console.log(await make().twice(), await tools.echo("x"));
