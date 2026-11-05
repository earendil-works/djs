let n1 = 0;
const sym2 = Symbol.for("shared");
interface I2 { a: number; b?: number }
type T2 = number | string;
function id<T>(x: T): T { return x; }
for (let i3 = 0; i3 < 3; i3++) {
  async function f4(a5: number): Promise<number> {
    console.log("p6", JSON.stringify((await tools.echo({ a: 9, b: a5 }))));
    try {
      throw new Error((await tools.echo("quo\"te")));
    } catch (e7) {
      console.log("c8", e7.message);
    }
    return id<number>(a5);
  }
  console.log("p9", JSON.stringify((await f4(n1))));
}
class C10 {
  constructor(public value: number) {}
  get twice() { return this.value * 2; }
  get v() { return this.value; }
  set v(x) { this.value = x; }
  get(x) { return this.value + x + n1; }
  async fetch(x) { return this.value + (await tools.echo(x)); }
  static make(v) { return new C10(v + 1); }
}
const o11 = new C10([(true ? n1 : 4), (await new Promise((res) => res(n1)))][0]);
n1 -= (await tools.echo({ v: (await tools.echo(3)) })).v;
const f12 = async (a13: number): Promise<number> => {
  if (([0, a13].includes(1) && a13 > a13)) {
    console.log("p15", JSON.stringify((await tools.echo((await Promise.all([3, 8].map(async (x14) => x14 * (await tools.echo([n1, n1][0])))))))));
  } else {
    ;
  }
  return ((false || true) ? (await tools.echo(7)) : a13);
};
console.log("p19", JSON.stringify((await o11.fetch([n1, 3].length))));
const { a: v20, b: v21 = 6 } = { a: (n1 as number), b: n1, m() { return this.a; } };
for (const x23 of (await tools.echo([5, v21]))) {
  L24: for (const i25 of [0, 1, 2]) {
    console.log("p26", JSON.stringify(Array.from(new Uint8Array([v21, 3, i25])).join("-")));
    if (i25 === 2) continue L24;
  }
  for (const x27 of [(x23 * 2 - x23), (await new Promise((res) => res(x23)))]) {
    "${x}";
  }
}
;
let v31: { a: number; b: number } = JSON.parse(JSON.stringify({ a: 4, b: n1, [sym2]: 6 }));
return { n: n1, vars: [n1, v20, v21, v31], calls: [(v20 + v20), C10.make(v20).get(v21), (await tools.echo(9))], o: { a: "s1".length, get b() { n1++; return n1; }, set b(v) { n1 = v; } }, last: await tools.echo([...new Set([n1, v21, 6])].join(",")) };
