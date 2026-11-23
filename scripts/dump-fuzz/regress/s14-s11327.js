let n1 = 0;
class C2 {
  constructor(v) { this.value = v; }
  get twice() { return this.value * 2; }
  get v() { return this.value; }
  set v(x) { this.value = x; }
  static count = n1;
  static { n1 += 1; }
  get(x) { return this.value + x + n1; }
  async fetch(x) { return this.value + (await tools.echo(x)); }
  async later(x) { const f = () => this.value + x; await tools.echo(x); return f(); }
  static make(v) { return new C2(v + 1); }
}
class D3 extends C2 {
  constructor(v) { super(v * 2); this.extra = v; }
  get(x) { return super.get(x) + this.extra; }
  async fetch(x) { const a = await super.fetch(x); return a + (await tools.echo(this.extra)); }
}
const o4 = new D3((await tools.echo((await (async () => (8))()))));
const [v5 = (n1, n1), ...v7] = [Math.max(n1, n1), ...[n1, ...[1, n1]]];
let [v8, , v9 = (await o4.fetch(n1))] = [3, 9].filter((x) => x > n1).map((x) => x + n1);
v7[v9 % 2] = (await tools.echo((n1 + 9)));
for (const x11 of [(await new Promise((res) => res(v8))), (await tools.echo(v9))]) {
  const v12 = new Set([(3 || v8), (await tools.echo([6, 9]))[1], (v8 ?? v9)]);
  for (const x13 of [(await tools.echo(6)), (await tools.echo([v9, v8]))[1]]) {
    if (!v5 > x11) {
      let v14 = String(x11);
      console.log("p15", JSON.stringify(v14));
      console.log("p16", JSON.stringify({ get a() { return v8 + 1; }, set a(v) { v8 = v; }, b: ((s, ...v) => s.length + v.length)`a${x11}b${9}` }));
    }
    n1 -= (await tools.echo({ v: (await tools.echo([8, v8]))[1] })).v;
  }
}
function* g17(k) {
  for (let i = 0; i < k; i++) yield i + n1;
}
for (const x18 of [{ a: 1, b: 1 }.b, v9].entries().map(([k, x]) => k + x)) {
  B19: {
    console.log("p20", JSON.stringify([...g17(2)]));
    if ((false && false)) break B19;
    for (let i21 = 0; i21 < 1; i21++) {
      n1 = Object.assign(Object.create(null), { a: 3, b: v8 }).a;
      n1 -= v5;
    }
  }
}
{
  using r22 = { [Symbol.dispose]() { console.log("u23"); } };
  try {
    L24: for (const i25 of [0, 1, 2]) {
      console.log("p26", JSON.stringify(String((v9 + 1))));
      let v27 = new Error(("a" + v8));
      console.log("p28", JSON.stringify([v27.name, v27.message, v27.extra ?? null]));
      if (i25 === 2) break L24;
    }
  } catch (e29) {
    console.log("c30", e29.message);
  } finally {
    console.log("p31", JSON.stringify({ a: v7.length, get b() { n1++; return n1; }, set b(v) { n1 = v; } }));
    L32: for (const i33 of [0, 1, 2]) {
      if ("a" in { a: v9, b: v5 }) continue;
      if (i33 === 2) break L32;
    }
  }
}
return { n: n1, vars: [n1, v5, v7, v8, v9], calls: [1, 2, (n1 ?? v9)], o: new Proxy({ a: [v8, 4][1], b: 4 }, { set(o, k, v) { n1++; o[k] = v; return true; } }) };
