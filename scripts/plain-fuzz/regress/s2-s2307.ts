let n1 = 0;
interface I1 { a: number; b?: number }
type T1 = number | string;
function id<T>(x: T): T { return x; }
for (let i2 = 0; i2 < 1; i2++) {
  const f3 = async (a4: number): Promise<number> => {
    let v5 = [...new Map(Object.entries({ a: i2, b: i2 })).keys()].join(",");
    const v6: Date = new Date(Date.UTC(2024, (a4 as number) % 12, [5, 2].length + 1));
    return new Error(v5).message.length;
  };
  console.log("p7", JSON.stringify((await f3(3))));
}
console.log("p8", JSON.stringify([n1, n1].join("-")));
let w9 = 0;
do {
  w9++;
  let n10 = (await tools.echo(7));
  function f11(x) { return n10 * 2 + x; }
  console.log("p12", JSON.stringify(f11(n1)));
} while (w9 < 2);
for (const x13 of (await tools.echo([n1, 7]))) {
  {
    using r14 = { [Symbol.dispose]() { console.log("u15"); } };
    for (const x16 of (await tools.echo([n1, x13]))) {
      const v17: Date = new Date(2 * 86400000);
      let v18: Error = new Error([..."a\\b"].reverse().join(""));
      console.log("p19", JSON.stringify([v18.name, v18.message, v18.extra ?? null]));
    }
  }
  "${x}";
  for (const x20 of [new Map(Object.entries({ a: 6, b: 4 })).size, 3]) {
    let v21: Uint8Array | Float64Array = new Uint8Array(new ArrayBuffer(4), 1, 2);
    console.log("p22", JSON.stringify(Array.from(v21)));
  }
}
for (const k23 in { get a() { return n1 + 1; }, set a(v) { n1 = v; }, b: n1 }) {
  let [v24, , v25 = n1] = (await tools.echo((await tools.echo([0, n1]))));
  class C27 {
    constructor(public value: number) {}
    get twice() { return this.value * 2; }
    get v() { return this.value; }
    set v(x) { this.value = x; }
    get(x) { return this.value + x + v25; }
    async fetch(x) { return this.value + (await tools.echo(x)); }
    static make(v) { return new C27(v + 1); }
  }
  const o28 = new C27((await tools.echo({ a: v24, b: 8 }.a)));
}
console.log("p29", JSON.stringify({ a: n1!, b: (9 > n1 ? n1 : 8), m() { return this.a; } }));
n1 -= (await tools.echo(n1));
if ((n1 * 2 - 6) > (n1 * 2 - n1)) {
  console.log("p30", JSON.stringify(Object.values((await tools.echo({ a: 4, b: n1 }))).filter((x) => typeof x === "number")));
  const v31: Uint8Array | Float64Array = Uint8Array.from([n1, n1].filter((x) => x > n1));
  let [v32, , v33 = 4] = [new Error("a\\b").message.length];
}
return { n: n1, vars: [n1], calls: [(await tools.echo({ v: n1 })).v, 6, (await tools.echo(n1))], o: (await tools.echo(({ a: n1, b: n1 } satisfies object))), last: await tools.echo(`${"x"}-${n1}:${"s1"}`) };
