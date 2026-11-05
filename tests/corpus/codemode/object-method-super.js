// An object literal's async method that uses `super` after an await: its continuation is
// called with the method's home's prototype (as a class method's), and the method itself
// is written in its literal, where its home is, also where its frame and other bindings
// name it (0215; it was refused: "a method that uses super outside its object literal").
const base = { label(n) { return "item " + n; }, tag: "B" };
const api = {
  __proto__: base,
  prefix: ">",
  async describe(n) {
    const a = await tools.echo(n);
    const b = await tools.echo(a + 1);
    return this.prefix + super.label(a + b) + super.tag;
  },
};
const box = { inner: { __proto__: base, async one(n) { const v = await tools.echo(n); return super.label(v); } } };
const list = [{ __proto__: base, async *pages(n) { yield await tools.echo(n); yield super.label(n + 1); } }];
const describe = api.describe;
const p = box.inner.one(3);
const it = list[0].pages(5);
console.log(await api.describe(1));
console.log(await p, (await it.next()).value, (await it.next()).value);
console.log(await describe.call(api, 2), describe === api.describe, Object.getPrototypeOf(api) === base);
