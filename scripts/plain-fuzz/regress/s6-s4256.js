let n1 = 0;
[...new Set([n1, n1, n1])].join(",");
console.log("p2", JSON.stringify((Math.max(n1, 1) * 2 - (await new Promise((res) => res(8))))));
{
  using r3 = { [Symbol.dispose]() { console.log("u4"); } };
  const v5 = new Set([[1, 0].length, (n1 > 4 ? n1 : 6), [4, 9].length]);
}
for (const x6 of [3, { a: 2, b: n1 }.a]) {
  for (const k7 in { ["b"]: 9, b: x6, a: x6, b: n1 }) {
    (await tools.echo(k7));
    console.log("p8", JSON.stringify(k7));
  }
}
(await tools.echo("x"));
for (let i9 = 0; i9 < 3; i9++) {
  const v10 = new Date(Date.UTC(2024, (i9 + i9) % 12, (await tools.echo([0, 9]))[1] + 1));
  console.log("p11", JSON.stringify(v10.toISOString()));
}
if (([n1, n1].includes(4) && (false || false))) {
  `${" "}-${7}:${"s1"}`;
  console.log("p12", JSON.stringify((await tools.echo((true ? "😀" : "a")))));
}
for (const k13 in { ...{ a: n1, b: 7 }, b: 0 }) {
  let w14 = 0;
  while (w14 < 2) {
    w14++;
    if ((false && false)) break;
  }
  async function* ag15(k) {
    for (let i = 0; i < k; i++) {
      yield await tools.echo(i * 4);
    }
  }
  for await (const x16 of ag15(1)) {
    const f17 = async (a18) => {
      console.log("p19", JSON.stringify({ a: 4, b: (await tools.echo([x16, a18]))[1] }));
      let v20 = new Date(Date.UTC(2024, (a18 || 7) % 12, { a: a18, b: 1 }.b + 1));
      console.log("p21", JSON.stringify(v20.toISOString()));
      v20.setUTCDate((await tools.echo(v20.getUTCDate())) + 1);
      console.log("p23", JSON.stringify(v20.toISOString()));
      return n1;
    };
    console.log("p24", JSON.stringify((await f17(x16))));
  }
}
return { n: n1, vars: [n1], calls: [(5 || n1), [n1, n1].length, n1], o: (await tools.echo(JSON.parse(JSON.stringify({ a: n1, b: n1 })))), last: await tools.echo(" ") };
