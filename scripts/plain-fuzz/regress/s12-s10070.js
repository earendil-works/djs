let n1 = 0;
const { a: v2, b: v3 = n1 } = { get a() { return n1 + 1; }, set a(v) { n1 = v; }, b: (await tools.echo(n1)) };
for (const k5 in { a: v3, get b() { n1++; return n1; }, set b(v) { n1 = v; } }) {
  console.log("p6", JSON.stringify((await tools.echo([(false ? n1 : n1), (6, n1)]))[1]));
  const { a: v7, b: v8 = (v2, v3) } = { ...{ get a() { return n1 + 1; }, set a(v) { n1 = v; }, b: 0 }, b: v3 };
}
class C10 {
  constructor(v) { this.value = v; }
  get twice() { return this.value * 2; }
  get v() { return this.value; }
  set v(x) { this.value = x; }
  static count = n1;
  static { n1 += 1; }
  get(x) { return this.value + x + n1; }
  async fetch(x) { return this.value + (await tools.echo(x)); }
  async later(x) { const f = () => this.value + x; await tools.echo(x); return f(); }
  static make(v) { return new C10(v + 1); }
}
const o11 = new C10(([v2, v2].includes(4) ? v2 : (await tools.echo(7))));
L12: for (const i13 of [0, 1, 2]) {
  if (!i13 === 7) {
    try {
      ;
    } catch (e17) {
      console.log("c18", e17.message);
      "b".repeat(3).padStart(8, "*");
    } finally {
      const { a: v19, b: v20 = [7, 4][0] } = { ["a"]: n1, a: (await tools.echo(v3)), a: [v2, 4][0], b: (n1 ?? i13) };
      const v22 = new Set([new Uint8Array([i13, 2, n1]).byteOffset, { a: v3, b: v3 }.b, 4]);
    }
  } else {
    try {
      console.log("p23", JSON.stringify((await tools.echo(JSON.parse(JSON.stringify({ a: v3, b: i13 }))))));
    } catch (e24) {
      console.log("c25", e24.message);
      let v26 = new Uint8Array(new ArrayBuffer(4), 1, 2);
    }
    for (const x27 of (await tools.echo([i13, n1]))) {
      let v28 = new Uint8Array([[n1, 6].length, new Map([["", 3], ["k", v3]]).size, (Math?.max?.(n1, await tools.echo(i13)) ?? 0)]);
      n1 = ({ a: 9, b: 6 }.a + (6 ?? n1));
    }
    const [v29 = 6, ...v31] = [(await tools.echo(9)), ...[i13, n1].filter((x) => x > n1)];
  }
  let w32 = 0;
  do {
    w32++;
    if ((await o11.later(v2)) > /(?<w>\w)\d/gu.lastIndex) {
      String(Object.assign(new RangeError("r"), { code: v3 }).cause);
    }
    if (i13 > 3) break;
  } while (w32 < 1);
  if (i13 === 2) continue L12;
}
B33: {
  ;
  if (2 > v3) break B33;
  L37: for (const i38 of [0, 1, 2]) {
    if (!true) break;
    L39: for (const i40 of [0, 1, 2]) {
      if ((false && v3 > 4)) continue;
      n1 += (v2 * 2 - [9, v2][0]);
      if (i40 === 0) break L39;
      n1 = [3, ...[i40, i38]].length;
    }
    if (i38 === 0) continue L37;
  }
  for (const x41 of [(v2 ?? n1), "😀".length]) {
    let v42 = 37n;
    v42 = v42 * 2n + BigInt((Math.max(n1, v2), [v2, v2].length) | 0);
    n1 = [(x41 || 3), (v3 ?? 6)][1];
  }
}
{
  await using r43 = { async [Symbol.asyncDispose]() { console.log("u44", await tools.echo("bye")); } };
  let n45 = [4, 7][0];
  const f46 = (x) => (n45 += x);
  console.log("p47", JSON.stringify(f46([n1, 7].length)));
}
const [v48 = (/x+/y.test("abab xx1") ? 1 : 0), ...v50] = (await Promise.all([v3, 2].filter((x) => x > v3).map(async (x51) => (await tools.echo(x51 + (o11.value = n1))))));
let v52 = [v2, ...Array.from(new Float64Array([4, 0.5]))];
return { n: n1, vars: [n1, v2, v3, v48, v50, v52], calls: [(0 + v2), (v48 ?? v2), (9 && v3)], o: { ...new Proxy({ a: n1, b: 2 }, { get: (o, k) => (k === "b" ? n1 : o[k]) }), b: 8 } };
