// Object literals' methods that use `super` after an await, capturing nothing: each is
// written in its home's literal, also where its frame or another binding names it
// (`alias`), nested holders and an async generator method included (0215).
const base = { hi(n) { return "hi" + n; }, tag: "B" };
const box = { inner: { __proto__: base, async m(n) { const a = await later(n); return super.hi(a) + super.tag; } } };
const list = [{ __proto__: base, async *g(n) { yield await later(n); yield super.hi(n + 1); } }];
const o = { __proto__: base, async m(n) { const a = await later(n); const b = await later(a + 1); return super.hi(a + b); } };
const alias = o.m;
async function main() {
  const p = box.inner.m(1);
  const it = list[0].g(5);
  print(await p, (await it.next()).value, (await it.next()).value);
  print(await alias.call(o, 2), alias === o.m);
}
main();
