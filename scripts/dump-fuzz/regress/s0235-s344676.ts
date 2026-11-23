let n1 = 0;
interface I1 { a: number; b?: number }
type T1 = number | string;
function id<T>(x: T): T { return x; }
class C2 {
  value: number;
  constructor(v: number) { this.value = v; }
  get twice() { return this.value * 2; }
  get v() { return this.value; }
  set v(x) { this.value = x; }
  static count = 0;
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
const o4 = new D3((Math.max(n1, n1) + (await tools.echo({ v: n1 })).v));
n1 += (await tools.echo(n1));
for (let i5 = 0; i5 < 3; i5++) {
  L6: for (const i7 of [0, 1, 2]) {
    const v8: number = ((s, ...v) => s.length + v.length)`a${(5 + 4)}b${i7}`;
    if (i7 === 0) break L6;
    console.log("p10", JSON.stringify((await Promise.all([6, i5].map(async (x9) => (await tools.echo(x9 + { a: n1, b: i7 }.a))))).join("-")));
  }
  const v11: Map<string, number> = new Map([[[-0, NaN, Infinity, i5 / 0].map((x) => (Object.is(x, -0) ? "-0" : String(x))).join(","), (i5 || i5)], ["k", "😀".length]]);
  console.log("p12", JSON.stringify([...v11]));
}
async function* ag13(k) {
  for (let i = 0; i < k; i++) {
    yield await tools.echo(i * 5);
  }
}
for await (const x14 of ag13(1)) {
  console.log("p15", JSON.stringify([x14, x14].map((x) => x + 7).length));
  {
    using r16 = { [Symbol.dispose]() { console.log("u17"); } };
    console.log("p18", JSON.stringify(new Date(Date.UTC(2024, n1 % 12, 2 + 1)).toISOString()));
    for (const x19 of [new Error("üñí").message.length, 3]) {
      const v20: Set<number> = new Set([(2 ?? n1), (x14 ?? n1), x14]);
      console.log("p21", JSON.stringify([...v20]));
      console.log("p22", JSON.stringify(Object.defineProperty([n1, 2], 2, { get: () => n1, enumerable: true, configurable: true }).map((x) => x + (x14 + 5))));
      let v23: Uint8Array | Float64Array = new Float64Array([new Map([["\ud800", 9], ["k", x19]]).size, 0.5]);
    }
  }
}
async function f24(a25: number): Promise<number> {
  switch (((a25 ?? 1) || (n1, n1)) % 3) {
    case 0: {
      let v26: Uint8Array | Float64Array = new Float64Array([(9 * 2 - 6), 0.5]);
      console.log("p27", JSON.stringify(Array.from(v26)));
      break;
    }
    case [n1, 9].length: {
      {
        using r28 = { [Symbol.dispose]() { console.log("u29"); } };
        n1 += (o4.v = Math.max(a25, a25));
      }
      try {
        try {
          String(new Error("\u0000z"));
          throw new Error(("" + n1));
          n1 = n1;
        } finally {
          const v30 = [a25, , 5].map((x) => x + 9);
          console.log("p31", JSON.stringify(v30));
        }
      } catch (e) {
        console.log("outer", e.message);
      }
      break;
    }
    case [a25, 3][0]: {
      const v32: number = n1;
      console.log("p33", JSON.stringify(v32));
      break;
    }
  }
  return ([n1, a25].length || 2);
}
async function f34(a35: number): Promise<number> {
  console.log("p36", JSON.stringify(/(?<w>\w)\d/gu.lastIndex));
  const v37: Map<string, number> = new Map([[("x" + a35), [9, n1][0]], ["k", (D3.count++)]]);
  return ("a" in { a: a35, b: 9 } ? { a: 6, b: 4 }.a : { a: a35, b: a35 }.b);
}
console.log("p38", JSON.stringify([...new Set([n1, n1])].join(",")));
async function f39(a40: number): Promise<number> {
  console.log("p41", JSON.stringify(Object.defineProperty([8, 2], 2, { get: () => n1, enumerable: true, configurable: true }).map((x) => x + a40)));
  for (const x42 of [(a40 * 2 - 6), (a40, a40)]) {
    console.log("p43", JSON.stringify(a40));
    {
      using r44 = { [Symbol.dispose]() { console.log("u45"); } };
      let v46: number[] = [x42, [n1, x42][1], x42!];
      console.log("p47", JSON.stringify(v46));
    }
  }
  return new Uint8Array(new ArrayBuffer(4), 1, 2).length;
}
return { n: n1, vars: [n1], calls: [[n1, n1][0], "\u0000z".length, (o4.value = n1)], o: ({ ...{ a: n1, b: 5 }, b: 6 } satisfies object) };
