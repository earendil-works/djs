let n1 = 0;
const sym2 = Symbol.for("shared");
interface I2 { a: number; b?: number }
type T2 = number | string;
function id<T>(x: T): T { return x; }
function* g3(k) {
  for (let i = 0; i < k; i++) yield i + n1;
}
let n4 = (n1, 4);
const f5 = (x) => { n4++; return n4 + x; };
const f6 = async (a7: number): Promise<number> => {
  for (let i8 = 0; i8 < 3; i8++) {
    let w9 = 0;
    while (w9 < 2) {
      w9++;
      console.log("p10", JSON.stringify(Object.values({ a: 5, b: a7 }).filter((x) => typeof x === "number")));
      console.log("p11", JSON.stringify((await tools.echo("${x}"))));
    }
    if (n1 > 1) break;
    const v12: Date = new Date(Date.UTC(2024, (await (async () => (7))()) % 12, (9 * 2 - 2) + 1));
    console.log("p13", JSON.stringify(v12.toISOString()));
  }
  for (const k14 in (await tools.echo({ a: n1, b: a7 }))) {
    console.log("p15", JSON.stringify((await tools.echo(f5(0)))));
    console.log("p16", JSON.stringify(k14));
  }
  for (const x17 of g3(3).map((x) => x * 2).filter((x) => x >= 0).take(2)) {
    const v18 = new Set([n4, ...[n4, 5]]);
    console.log("p19", JSON.stringify([...v18]));
    for (let i20 = 0; i20 < 1; i20++) {
      const [v21 = "üñí".length, ...v23] = (await Promise.all((await Promise.all([x17, n1].map(async (x25) => (await tools.echo(x25 + [n4, 4][0]))))).map(async (x24) => (await tools.echo(x24 + (i20 * 2 - n1))))));
      ;
      v23.push(1);
    }
    console.log("p29", JSON.stringify({ a: [x17, 6].length, b: (/a(b)?/g.test("abab xx1") ? 1 : 0) }));
  }
  return (7 > n4 ? (Math?.max?.(9, await tools.echo(a7)) ?? 0) : { a: 6, b: n1 }.a);
};
console.log("p30", JSON.stringify((await f6((2, n4)))));
for (const k31 in { a: 1, b: n1, get g() { return n1 * 2; } }) {
  let v32: RegExp = new RegExp("x+", "g");
}
for (let i33 = 0; i33 < 1; i33++) {
  ("quo\"te" + 7);
}
const v34 = new Date(n1 * 86400000);
let v35 = Uint8Array.from((await tools.echo([n1, n4])));
{
  await using r36 = { async [Symbol.asyncDispose]() { console.log("u37", await tools.echo("bye")); } };
  v34.setUTCHours(n1);
  console.log("p38", JSON.stringify(v34.toISOString()));
}
return { n: n1, vars: [n1, n4, v34.toISOString(), Array.from(v35)], calls: [n4, [5, 5].reduce((s, x) => s + x, n4), n1], o: new Proxy({ a: (6 + 1), b: 5 }, { get: (o, k) => (k === "b" ? n1 : o[k]) }) };
