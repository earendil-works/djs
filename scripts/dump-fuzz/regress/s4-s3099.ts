let n1 = 0;
interface I1 { a: number; b?: number }
type T1 = number | string;
function id<T>(x: T): T { return x; }
n1 += n1!;
`${"</script>"}-${n1}:${" "}`;
for (let i2 = 0; i2 < 2; i2++) {
  if ("x" !== "</script>") break;
}
class C3 {
  constructor(public value: number) {}
  get twice() { return this.value * 2; }
  get v() { return this.value; }
  set v(x) { this.value = x; }
  get(x) { return this.value + x + n1; }
  async fetch(x) { return this.value + (await tools.echo(x)); }
  static make(v) { return new C3(v + 1); }
}
const o4 = new C3((await tools.echo("üñí")).length);
{
  using r5 = { [Symbol.dispose]() { console.log("u6"); } };
  let w7 = 0;
  while (w7 < 1) {
    w7++;
    for (const k8 in ({ a: n1, b: 9 } satisfies object)) {
      if ("a" in { a: 1, b: n1 }) break;
    }
  }
  for (const x9 of (await tools.echo([n1, n1]))) {
    for (let i10 = 0; i10 < 2; i10++) {
      console.log("p11", JSON.stringify([...new Set([n1, 3, i10])]));
    }
    [..."üñí"].reverse().join("");
    for (const k12 in { ["a"]: x9, a: 2, a: x9, b: 7 }) {
      console.log("p13", JSON.stringify((await tools.echo((await tools.echo(k12))))));
    }
  }
}
n1 += [new Date(Date.UTC(2024, n1 % 12, 6 + 1)).getUTCMonth(), new Date(Date.UTC(2024, n1 % 12, 6 + 1)).getUTCDate()].reduce((s, x) => s + x, [n1, n1][1]);
for (const x14 of [[n1, 0].length, (true ? 2 : n1)].entries().map(([k, x]) => k + x)) {
  async function f15(a16: number): Promise<number> {
    let v17 = Object.assign(new RangeError("r"), { code: new C3(x14).twice });
    console.log("p18", JSON.stringify([v17.name, v17.message, v17.extra ?? null]));
    v17.extra = [v17.message.length].reduce((s, x) => s + x, (8 + x14));
    return (await tools.echo({ a: x14, b: 7 })).a;
  }
  console.log("p19", JSON.stringify(new C3((4 + x14)).twice));
  for (const x20 of [4, (n1 > n1 ? 8 : 6)]) {
    function* g21(k) {
      for (let i = 0; i < k; i++) yield i + x14;
    }
    console.log("p22", JSON.stringify({ a: (await tools.echo(n1)), get b() { n1++; return n1; }, set b(v) { n1 = v; } }));
  }
}
{
  await using r23 = { async [Symbol.asyncDispose]() { console.log("u24", await tools.echo("bye")); } };
  for (let i25 = 0; i25 < 1; i25++) {
    let n26 = i25;
    function f27(x) { return n26 * 2 + x; }
    console.log("p28", JSON.stringify(f27((await new Promise((res) => res(4))))));
    const { a: v29, b: v30 = n1 } = ((await tools.echo({ a: 3, b: n1 })) satisfies object);
    L32: for (const i33 of [0, 1, 2]) {
      const { a: v34, b: v35 = (i25 && n1) } = { a: 0, b: C3.make(i25).get(v29), m() { return this.a; } };
      (await tools.echo("üñí"));
      if (i33 === 1) continue L32;
    }
  }
  console.log("p37", JSON.stringify(((await tools.echo({ v: 2 })).v || (n1, n1))));
}
return { n: n1, vars: [n1], calls: [(await tools.echo(n1)), (n1 && n1), Math.max(6, n1)], o: { a: Math.max(n1, n1), get b() { n1++; return n1; }, set b(v) { n1 = v; } }, last: await tools.echo("b") };
