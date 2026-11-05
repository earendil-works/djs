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
    static count = v25;
    get(x) { return this.value + x + v25; }
    async fetch(x) { return this.value + (await tools.echo(x)); }
    static make(v) { return new C27(v + 1); }
  }
  const o28 = new C27({ ["a"]: 8, a: 8, a: 3, b: 6 }.b);
}
switch (n1! % 3) {
  case 0: {
    function* g29(k) {
      for (let i = 0; i < k; i++) yield i + n1;
    }
    break;
  }
  case [n1, n1][0]: {
    if ((2 as number) === (n1 + 1)) {
      L30: for (const i31 of [0, 1, 2]) {
        ;
        if (i31 === 1) continue L30;
      }
      const v35: Uint8Array | Float64Array = Uint8Array.from([n1, n1].filter((x) => x > n1));
    } else {
      for (const k36 in { a: 9, b: 1, m() { return this.a; } }) {
        let [v37, , v38 = (await tools.echo([6, n1]))[1]] = [1, ...[n1, n1]].filter((x) => x > [n1, n1].length);
        console.log("p40", JSON.stringify({ get a() { return n1 + 1; }, set a(v) { n1 = v; }, b: (await tools.echo(v37)) }));
      }
    }
    for (const x41 of [n1!, 0].entries().map(([k, x]) => k + x)) {
      try {
        await tools.fail({ message: "😀" });
        console.log("p42", JSON.stringify({ ...(await tools.echo({ a: x41, b: x41 })), b: "üñí".length }));
      } catch (e43) {
        console.log("c44", e43.message);
      }
    }
    break;
  }
  case (9 && n1): {
    (await tools.echo(" "));
    console.log("p46", JSON.stringify((await Promise.all([2, n1].map(async (x45) => (await tools.echo(x45 + (await tools.echo({ v: x45 })).v))))).length));
    // falls through
  }
  default: {
    let v47 = new RegExp("x+", "g");
  }
}
try {
} catch (e48) {
  console.log("c49", e48.message);
  async function* ag50(k) {
    for (let i = 0; i < k; i++) {
      yield await tools.echo(i * 2);
    }
  }
  for await (const x51 of ag50(1)) {
    "a";
    console.log("p52", JSON.stringify([x51, x51, 1].map((x) => x + n1!)));
  }
}
for (let i53 = 0; i53 < 3; i53++) {
  try {
    console.log("p54", JSON.stringify([n1, 2].filter((x) => x > n1).map((x) => x + { a: n1, b: 8 }.b)));
    " ";
  } catch (e55) {
    console.log("c56", e55.message);
    let w57 = 0;
    while (w57 < 1) {
      w57++;
      const v58: number[] = [...new Set([n1, i53, 0])];
      console.log("p59", JSON.stringify(v58));
      if (i53 > 2) break;
      v58.push(Object.values({ a: n1, b: i53 }).filter((x) => typeof x === "number").length);
    }
  }
}
return { n: n1, vars: [n1], calls: [[n1, n1][1], (/./s.test("abab xx1") ? 1 : 0), [n1, 9].reduce((s, x) => s + x, n1)], o: { a: (3 + 7), b: (5 * 2 - 4) }, last: await tools.echo(`${" "}-${n1}:${"</script>"}`) };
