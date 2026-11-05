// In language/, not codemode/: the slim build has no BigInt or `using` (D2411).
// A subclass whose base class is placed in a deeper block of K (it reads a binding of
// the loop's body): the subclass's statement reads the base's local, so it goes no higher
// than the base (0215; plain-dump fuzzer seed 668041, --lazy; also on main before).
let n1 = 0;
const sym2 = Symbol("k");
let [v3, , v4 = (n1, 7)] = [9, 7].filter((x) => x > n1).filter((x) => x > [8, 6][1]);
for (let i6 = 0; i6 < 2; i6++) {
  v4 -= (new Date(Date.UTC(2024, 7 % 12, 9 + 1)).getTime() / 86400000);
  let v7 = (await tools.echo((await tools.echo([2, v4]))));
}
console.log("p8", JSON.stringify({ a: (await tools.echo([v4, n1]))[1], b: (n1 * 2 - n1), get g() { return v4 * 2; } }));
try {
  await tools.fail({ message: ("" + v4) });
  const v9 = new Map([[String(Object.assign(new RangeError("r"), { code: 0 })), (v4 ?? 6)], ["k", "\u0000z".length]]);
  console.log("p10", JSON.stringify([...v9]));
} catch (e11) {
  console.log("c12", e11.message);
} finally {
  let v13 = new Map(Object.entries(Object.assign(Object.create(null), { a: v4, b: v3 })));
}
{
  await using r14 = { async [Symbol.asyncDispose]() { console.log("u15", await tools.echo("bye")); } };
  let v16 = new Map(Object.entries({ a: 0, b: n1 }));
}
if (![n1, 2].includes(v4)) {
  [..."quo\"te"].reverse().join("");
  JSON.stringify({ a: v3, b: 5 });
} else {
  for (const x17 of [(Math?.max?.(v3, await tools.echo(n1)) ?? 0), (await new Promise((res) => res(5)))]) {
    for (const x18 of [(await tools.echo(v3)), (await tools.echo(x17))]) {
      "\u0000z";
    }
    console.log("p19", JSON.stringify((await tools.echo([..."line\nbreak"].reverse().join("")))));
  }
}
for (const x20 of (await tools.echo([n1, 5]))) {
  if (("s1" !== "" || [0, 4].includes(v4))) {
    let v21 = new Map(Object.entries({ a: v3, b: v4 }));
  }
  if ("a" in { a: 0, b: v3 }) break;
}
async function* ag22(k) {
  for (let i = 0; i < k; i++) {
    yield await tools.echo(i * 3);
  }
}
for await (const x23 of ag22(2)) {
  let n24 = (await tools.echo(n1));
  function f25(x) { return n24 * 2 + x; }
  console.log("p26", JSON.stringify(f25(x23)));
  class C27 {
    constructor(v) { this.value = v; }
    get twice() { return this.value * 2; }
    get v() { return this.value; }
    set v(x) { this.value = x; }
    static count = v4;
    static { n24 += 1; }
    get(x) { return this.value + x + n24; }
    async fetch(x) { return this.value + (await tools.echo(x)); }
    async later(x) { const f = () => this.value + x; await tools.echo(x); return f(); }
    static make(v) { return new C27(v + 1); }
  }
  class D28 extends C27 {
    constructor(v) { super(v * 2); this.extra = v; }
    get(x) { return super.get(x) + this.extra; }
    async fetch(x) { const a = await super.fetch(x); return a + (await tools.echo(this.extra)); }
  }
  const o29 = new D28((await tools.echo([(await (async () => (4))()), [4, v4].reduce((s, x) => s + x, v4)]))[1]);
}
return { n: n1, vars: [n1, v3, v4], calls: [[5, n1].length, new Error("ok").message.length, (v4 || 9)], o: new Proxy({ a: (await tools.echo(v3)), b: (await tools.echo({ v: 0 })).v }, {}), last: await tools.echo([-0, NaN, Infinity, 3 / 0].map((x) => (Object.is(x, -0) ? "-0" : String(x))).join(",")) };
