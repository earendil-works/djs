// Found by the plain-dump fuzzer (0211): an async method of a subclass that awaits
// `super.fetch(...)` and then a tool, dumped again at the second await.
class C {
  constructor(v) { this.value = v; }
  async fetch(x) { return this.value + (await tools.echo(x)); }
}
class D extends C {
  constructor(v) { super(v * 2); this.extra = v; }
  async fetch(x) { const a = await super.fetch(x); return a + (await tools.echo(this.extra)); }
}
const o = new D(1);
console.log(await o.fetch(2));
return "done";
