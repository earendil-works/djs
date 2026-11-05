let n1 = 0;
console.log("p2", JSON.stringify([n1, n1].filter((x) => x > 3).map((x) => x + 0)));
async function* ag3(k) {
  for (let i = 0; i < k; i++) {
    yield await tools.echo(i * 3);
  }
}
for await (const x4 of ag3(1)) {
  for (let i5 = 0; i5 < 2; i5++) {
    let [v6, , v7 = (i5 ?? i5)] = (await Promise.all([new Date(Date.UTC(2024, x4 % 12, x4 + 1)).getUTCMonth(), new Date(Date.UTC(2024, x4 % 12, x4 + 1)).getUTCDate()].map(async (x9) => (await tools.echo(x9 + (new Date(i5 * 86400000).getTime() / 86400000))))));
  }
  console.log("p10", JSON.stringify({ a: (0 + n1), b: n1, m() { return this.a; } }));
}
try {
  ;
  await tools.fail({ message: (await tools.echo("😀")) });
  new Uint8Array([n1, n1, n1]).join(":");
} catch (e14) {
  console.log("c15", e14.message);
}
console.log("p16", JSON.stringify((true ? "b" : "😀").repeat(3).padStart(8, "*")));
const v17 = (await tools.echo(JSON.parse(JSON.stringify({ a: 4, b: n1 }))));
console.log("p18", JSON.stringify(v17));
const v19 = new Map([[JSON.stringify(v17), (new Date(Date.UTC(2024, 8 % 12, n1 + 1)).getTime() / 86400000)], ["k", 3]]);
v19.set([n1, n1].join("-"), ((1 || 0) + (v19.get("k") ?? 0)));
console.log("p20", JSON.stringify([...v19]));
v17[new Uint8Array(new ArrayBuffer(4), 1, 2).join(":") || "a"] = Math.max((await tools.echo(0)), n1);
return { n: n1, vars: [n1, v17, [...v19]], calls: [[n1, n1].length, n1, n1], o: { a: (n1 + n1), b: n1 }, last: await tools.echo([..."s1"].reverse().join("")) };
