let n1 = 0;
class C2 {
  constructor(v) { this.value = v; }
  get twice() { return this.value * 2; }
  get v() { return this.value; }
  set v(x) { this.value = x; }
  static count = 7;
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
const o4 = new D3((n1 === 1 ? (await tools.echo(n1)) : (true ? 0 : n1)));
async function f5(a6) {
  switch ((await tools.echo({ v: (await new Promise((res) => res(a6))) })).v % 3) {
    case 0: {
      {
        await using r7 = { async [Symbol.asyncDispose]() { console.log("u8", await tools.echo("bye")); } };
        n1 -= (Math?.max?.((await tools.echo(n1)), await tools.echo(Math.max(n1, 2))) ?? 0);
        const { a: v9, b: v10 = D3.make(n1).get(a6) } = { a: (a6 + 9), get b() { n1++; return n1; }, set b(v) { n1 = v; } };
      }
      if ("a" in { a: n1, b: 9 }) break;
      break;
    }
  }
  return 0;
}
let w12 = 0;
do {
  w12++;
  n1 = Math.max({ a: n1, b: 5 }.b, [1, n1].length);
  class C13 {
    constructor(v) { this.value = v; }
    get twice() { return this.value * 2; }
    get v() { return this.value; }
    set v(x) { this.value = x; }
    static count = n1;
    static { n1 += 1; }
    get(x) { return this.value + x + n1; }
    async fetch(x) { return this.value + (await tools.echo(x)); }
    async later(x) { const f = () => this.value + x; await tools.echo(x); return f(); }
    static make(v) { return new C13(v + 1); }
  }
  const o14 = new C13(((2, n1) || (n1 * 2 - n1)));
  let w15 = 0;
  while (w15 < 1) {
    w15++;
    B16: {
      `${"b"}-${6}:${""}`;
      console.log("p17", JSON.stringify((await new Promise((res) => res(D3.make(n1).get(0))))));
      if ("a" in { a: 2, b: n1 }) break B16;
      let v18 = new Uint8Array([(4 + n1), (await new Promise((res) => res(n1))), (true ? 8 : n1)]);
    }
  }
} while (w12 < 2);
let v19 = new Map([[(await tools.echo("</script>")), (3 && 3)], ["k", (await f5(n1))]]);
console.log("p20", JSON.stringify([...v19]));
for (let i21 = 0; i21 < 1; i21++) {
  v19.delete("k");
}
const f22 = async (a23) => {
  `${"s1"}-${n1}:${""}`;
  return (8 && (await tools.echo([n1, 1]))[1]);
};
console.log("p24", JSON.stringify((await f5((n1 + n1)))));
`${"</script>"}-${0}:${"${x}"}`;
console.log("p25", JSON.stringify({ a: [6, 2].reduce((s, x) => s + x, 0), b: (n1 + 5), *[Symbol.iterator]() { yield this.a; yield this.b; } }));
return { n: n1, vars: [n1, [...v19]], calls: [(n1, 2), (new Date(Date.UTC(2024, 0 % 12, 7 + 1)).getTime() / 86400000), new Error("a").message.length], o: { a: 8, b: n1 }, last: await tools.echo(("a\\b" + n1)) };
