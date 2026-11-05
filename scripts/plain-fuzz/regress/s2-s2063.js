let n1 = 0;
const sym2 = Symbol();
class C3 {
  constructor(v) { this.value = v; }
  get twice() { return this.value * 2; }
  get v() { return this.value; }
  set v(x) { this.value = x; }
  get(x) { return this.value + x + n1; }
  async fetch(x) { return this.value + (await tools.echo(x)); }
  static make(v) { return new C3(v + 1); }
}
const o4 = new C3((await tools.echo({ a: 3, b: n1 })).b);
try {
  console.log("p5", JSON.stringify((await tools.echo({ a: 0, b: n1, get g() { return n1 * 2; } }))));
} catch (e6) {
  console.log("c7", e6.message);
} finally {
  n1 += "üñí".length;
}
console.log("p9", JSON.stringify((await tools.echo((await Promise.all([n1, 8].map(async (x8) => (await tools.echo(x8 + "s1".length)))))))));
class C10 {
  constructor(v) { this.value = v; }
  get twice() { return this.value * 2; }
  get v() { return this.value; }
  set v(x) { this.value = x; }
  get(x) { return this.value + x + n1; }
  async fetch(x) { return this.value + (await tools.echo(x)); }
  static make(v) { return new C10(v + 1); }
}
const o11 = new C10(o4.twice);
async function* ag12(k) {
  for (let i = 0; i < k; i++) {
    yield await tools.echo(i * 3);
  }
}
for await (const x13 of ag12(1)) {
  function* g14(k) {
    for (let i = 0; i < k; i++) yield i + x13;
  }
  L15: for (const i16 of [0, 1, 2]) {
    n1 = (new Map(Object.entries({ a: 7, b: 9 })).get("k") ?? 0);
    console.log("p17", JSON.stringify({ a: ({ a: n1, b: x13 }[sym2] ?? 0), b: [x13, n1].reduce((s, x) => s + x, i16), m() { return this.a; } }));
    if (i16 === 1) continue L15;
  }
}
class C18 {
  constructor(v) { this.value = v; }
  get twice() { return this.value * 2; }
  get v() { return this.value; }
  set v(x) { this.value = x; }
  get(x) { return this.value + x + n1; }
  async fetch(x) { return this.value + (await tools.echo(x)); }
  static make(v) { return new C18(v + 1); }
}
const o19 = new C18(((4 + n1) * 2 - (true ? n1 : n1)));
;
(await tools.echo(" "));
return { n: n1, vars: [n1], calls: [n1, Math.max(0, n1), /^l/m.lastIndex], o: { a: Math.max(n1, 5), b: Math.max(n1, n1) }, last: await tools.echo((typeof n1)) };
