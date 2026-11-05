let n1 = 0;
const sym2 = Symbol();
class C3 {
  constructor(v) { this.value = v; }
  get twice() { return this.value * 2; }
  get v() { return this.value; }
  set v(x) { this.value = x; }
  static count = 7;
  get(x) { return this.value + x + n1; }
  async fetch(x) { return this.value + (await tools.echo(x)); }
  static make(v) { return new C3(v + 1); }
}
const o4 = new C3({ ...{ a: n1, b: 1 }, b: n1 }.b);
for (const k5 in { ["b"]: 9, b: 0, a: 3, b: n1 }) {
  n1 += String(2).length;
}
async function f6(a7) {
  const { a: v8, ...v10 } = await tools.echo({ a: (await tools.echo(3)), b: (await o4.fetch(a7)) });
  return (await tools.echo({ v: { a: 3, b: v8 }.b })).v;
}
console.log("p11", JSON.stringify((await f6((await tools.echo([8, 4]))[1]))));
console.log("p12", JSON.stringify({ a: ({ a: n1, b: n1 }[sym2] ?? 0), b: [n1, n1].reduce((s, x) => s + x, n1), m() { return this.a; } }));
for (const k13 in { ...{ a: n1, b: 4 }, b: n1 }) {
  console.log("p14", JSON.stringify((await tools.echo((await tools.echo({ a: n1, b: 4 }))))));
}
let n15 = new Date(Date.UTC(2024, n1 % 12, n1 + 1)).getUTCDate();
const f16 = (x) => (n15 += x);
if ("a" in (await tools.echo({ a: 4, b: n15 }))) {
  for (let i17 = 0; i17 < 3; i17++) {
    {
      using r18 = { [Symbol.dispose]() { console.log("u19"); } };
      [...new Map(Object.entries({ a: 1, b: 6 })).keys()].join(",");
      n15 -= (await new Promise((res) => res(n15)));
    }
  }
  console.log("p20", JSON.stringify((await f6((await (async () => (6))())))));
  let n21 = (7 + n15);
  function f22(x) { return n21 * 2 + x; }
  console.log("p23", JSON.stringify(f22((await tools.echo({ v: n21 })).v)));
}
class C24 {
  constructor(v) { this.value = v; }
  get twice() { return this.value * 2; }
  get v() { return this.value; }
  set v(x) { this.value = x; }
  static count = 0;
  get(x) { return this.value + x + n15; }
  async fetch(x) { return this.value + (await tools.echo(x)); }
  static make(v) { return new C24(v + 1); }
}
const o25 = new C24((new Date(Date.UTC(2024, n15 % 12, n1 + 1)).getTime() / 86400000));
return { n: n1, vars: [n1, n15], calls: [(await tools.echo({ v: n1 })).v, (false ? 0 : 1), (o4.v = n15)], o: { a: (await tools.echo({ v: n15 })).v, b: o4.get(n15) }, last: await tools.echo(String(2)) };
