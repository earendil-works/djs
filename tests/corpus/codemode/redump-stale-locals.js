// Bindings a later loop iteration has not declared yet, whose locals still hold the
// earlier iteration's values (no TDZ marker is needed there): a dump leaves them out
// (0215; pjz's class with an async method in a nested loop was refused for the first
// iteration's class).
for (const i of [0, 1]) for (let j = 0; j < 1; j++) {
  let n = i;
  class C { get(x) { return x + n; } async fetch(x) { return await tools.echo(x); } }
  const o = new C(i > 0 ? await tools.echo(4) : 4); console.log(o.get(1));
  const p = { v: i }, q = i > 0 ? await tools.echo(p.v) : 0, { r } = { r: i > 0 ? await tools.echo(5) : 5 };
  console.log(p.v, q, r);
}
for (const k of [0, 1, 2]) {
  if (k > 0) console.log("before", await tools.echo(k));
  const obj = { k, f() { return k; } };
  console.log(obj.f());
}
