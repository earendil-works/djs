let n1 = 0;
for (const x2 of (await tools.echo([n1, 8]))) {
  function* g3(k) {
    for (let i = 0; i < k; i++) yield i + n1;
  }
}
try {
  JSON.stringify({ a: 0, b: n1 });
  for (const x4 of [(await tools.echo(9)), (await tools.echo(n1))].entries().map(([k, x]) => k + x)) {
    let n5 = 1;
    const f6 = (x) => (n5 += x);
    class C7 {
      constructor(v) { this.value = v; }
      get twice() { return this.value * 2; }
      get v() { return this.value; }
      set v(x) { this.value = x; }
      get(x) { return this.value + x + n5; }
      async fetch(x) { return this.value + (await tools.echo(x)); }
      static make(v) { return new C7(v + 1); }
    }
    const o8 = new C7(new C7((await tools.echo([x4, 4]))[1]).twice);
    n1 = (await tools.echo((await tools.echo(n1))));
  }
} catch (e9) {
  console.log("c10", e9.message);
}
const v11 = new RegExp("[ab]", "g");
const f12 = async (a13) => {
  v11.lastIndex = new Date(Date.UTC(2024, a13 % 12, 9 + 1)).getUTCDate() % 3;
  console.log("p14", JSON.stringify([...new Map([["</script>", 8], ["k", 2]]).keys()].join(",")));
  v11.exec("abab xx1 l");
  console.log("p15", JSON.stringify([v11.source, v11.flags, v11.lastIndex]));
  return (new Uint8Array([a13, n1, a13])[0] ?? 0);
};
console.log("p16", JSON.stringify((await f12((await f12(5))))));
for (const k17 in { a: 3, b: n1 }) {
  const { a: v18, b: v19 = (n1 && n1) } = { a: [6, n1].length, get b() { n1++; return n1; }, set b(v) { n1 = v; } };
}
let v21 = { a: (await f12(3)), get b() { n1++; return n1; }, set b(v) { n1 = v; } };
let v22 = { a: n1, get b() { n1++; return n1; }, set b(v) { n1 = v; } }.a;
console.log("p23", JSON.stringify(v22));
const [v24 = (new Map(Object.entries(v21)).get("k") ?? 0), ...v26] = (await Promise.all((await tools.echo([v22, 7])).map(async (x27) => x27 * (await tools.echo([n1, 0].length)))));
return { n: n1, vars: [n1, [v11.source, v11.flags, v11.lastIndex], v21, v22, v24, v26], calls: ["quo\"te".length, (v24 && 6), 6], o: { a: 6, get b() { n1++; return n1; }, set b(v) { n1 = v; } } };
