let n1 = 0;
const sym2 = Symbol("k");
interface I2 { a: number; b?: number }
type T2 = number | string;
function id<T>(x: T): T { return x; }
L3: for (const i4 of [0, 1, 2]) {
  let n5 = n1;
  function f6(x) { return n5 * 2 + x; }
  if (i4 === 2) continue L3;
  function* g7(k) {
    for (let i = 0; i < k; i++) yield i + i4;
  }
}
const { a: v8, b: v9 = [6, n1].reduce((s, x) => s + x, n1) } = { a: (await tools.echo(n1)), b: (await tools.echo({ v: n1 })).v };
const { a: v11, b: v12 = (v9, v9) } = { a: v8, b: n1!, m() { return this.a; } };
console.log("p14", JSON.stringify((new Date(Date.UTC(2024, v12 % 12, v9 + 1)).getTime() / 86400000)));
n1 += 3;
async function* ag15(k) {
  for (let i = 0; i < k; i++) {
    yield await tools.echo(i * 1);
  }
}
for await (const x16 of ag15(1)) {
  try {
    await tools.fail({ message: [..."ok"].reverse().join("") });
    for (const x17 of (await tools.echo([v8, 7]))) {
      let v18 = 8;
      console.log("p19", JSON.stringify("x"));
    }
  } catch (e20) {
    console.log("c21", e20.message);
  }
  {
    await using r22 = { async [Symbol.asyncDispose]() { console.log("u23", await tools.echo("bye")); } };
    if ((true || true)) continue;
  }
}
(await tools.echo("${x}"));
let w24 = 0;
while (w24 < 1) {
  w24++;
  console.log("p25", JSON.stringify((await tools.echo([v9, 9][1]))));
  try {
    await tools.fail({ message: [0, v8].join("-") });
    n1 += v9;
  } catch (e26) {
    console.log("c27", e26.message);
    switch (new Set([6, v12]).size % 3) {
      case 0: {
        console.log("p28", JSON.stringify(({ a: 1, b: 1 }.a + [v12, v8].reduce((s, x) => s + x, v8))));
        // falls through
      }
      case (7 ?? n1): {
        const [v29 = "x".length, ...v31] = [(await tools.echo(v9)), ...[n1, n1]];
        break;
      }
      default: {
        let v32: Date = new Date((4 || v9) * 86400000);
      }
    }
  }
  console.log("p34", JSON.stringify((await Promise.all([8, 9, n1].map(async (x33) => x33 * (await tools.echo((new Map([["${x}", 4], ["k", x33]]).get("k") ?? 0))))))));
}
return { n: n1, vars: [n1, v8, v9, v11, v12], calls: [n1, { a: v12, b: v8 }.b, (await tools.echo(2))], o: { ...JSON.parse(JSON.stringify({ a: v9, b: v9 })), b: v9 }, last: await tools.echo("üñí") };
