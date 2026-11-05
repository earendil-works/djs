let n1 = 0;
if (![n1, n1].includes(1)) {
  let [v2, , v3 = "${x}".length] = (await tools.echo([4, ...[n1, 4]]));
  const v5 = new Map(Object.entries({ a: v3, b: v3 }));
  for (let i6 = 0; i6 < 3; i6++) {
    const f7 = async (a8) => {
      console.log("p9", JSON.stringify({ a: v2, b: v2 }.b));
      for (let i10 = 0; i10 < 3; i10++) {
        v5.set([..." "].reverse().join(""), 5);
        console.log("p11", JSON.stringify([...v5]));
      }
      {
        await using r12 = { async [Symbol.asyncDispose]() { console.log("u13", await tools.echo("bye")); } };
        console.log("p14", JSON.stringify("a"));
        const v15 = new Uint8Array([new Error("").message.length, (v2 + i6), (await tools.echo({ v: 1 })).v]);
      }
      return [6, ...[i6, 9]].length;
    };
  }
} else {
  let v16 = new Map(Object.entries({ ["b"]: n1, b: 7, a: 1, b: 8 }));
  console.log("p17", JSON.stringify((await tools.echo([n1, n1, 9]))));
}
const { a: v18, b: v19 = (n1 * 2 - 3) } = { a: (await tools.echo(4)), b: (0 > n1 ? 9 : 6), get g() { return n1 * 2; } };
("quo\"te" + 6);
class C21 {
  constructor(v) { this.value = v; }
  get twice() { return this.value * 2; }
  get v() { return this.value; }
  set v(x) { this.value = x; }
  static count = v19;
  static { n1 += 1; }
  get(x) { return this.value + x + n1; }
  async fetch(x) { return this.value + (await tools.echo(x)); }
  static make(v) { return new C21(v + 1); }
}
const o22 = new C21((v19 > v18 ? new Set([v18, v19]).size : v19));
for (let i23 = 0; i23 < 3; i23++) {
  console.log("p24", JSON.stringify(n1));
}
L25: for (const i26 of [0, 1, 2]) {
  for (let i27 = 0; i27 < 1; i27++) {
    let n28 = Math.max(v18, v19);
    function f29(x) { return n28 * 2 + x; }
    console.log("p30", JSON.stringify(f29((i27, v19))));
    class C31 {
      constructor(v) { this.value = v; }
      get twice() { return this.value * 2; }
      get v() { return this.value; }
      set v(x) { this.value = x; }
      static count = v19;
      static { n28 += 1; }
      get(x) { return this.value + x + n28; }
      async fetch(x) { return this.value + (await tools.echo(x)); }
      static make(v) { return new C31(v + 1); }
    }
    const o32 = new C31(((7 + 6) && (await tools.echo(4))));
  }
  if ("a" in { get a() { return n1 + 1; }, set a(v) { n1 = v; }, b: 0 }) {
    (typeof v18);
    console.log("p34", JSON.stringify((await Promise.all([6, v19].filter((x) => x > v19).map(async (x33) => x33 * (await tools.echo((o22.v = v18))))))));
    n1 += [i26, v18].join("-").length;
  } else {
    let v35 = [(n1 ?? 7), ...[v19, 9].map((x) => x + v19)];
    console.log("p36", JSON.stringify({ ...{ a: 6, b: 1, get g() { return n1 * 2; } }, b: (true ? v19 : v18) }));
    console.log("p37", JSON.stringify({ a: (4, 8), get b() { n1++; return n1; }, set b(v) { n1 = v; } }));
  }
  if (i26 === 1) continue L25;
}
const v38 = new Date(Date.UTC(2024, (await new Promise((res) => res(n1))) % 12, v19 + 1));
console.log("p39", JSON.stringify(v38.toISOString()));
for (let i40 = 0; i40 < 1; i40++) {
  let [v41, , v42 = (new Map(Object.entries({ a: n1, b: i40 })).get("k") ?? 0)] = (await Promise.all([i40, ...[8, 7]].map(async (x44) => (await tools.echo(x44 + (x44 || 4))))));
  L45: for (const i46 of [0, 1, 2]) {
    v38.setUTCDate((await tools.echo([(await tools.echo(8)), n1]))[1] + 1);
    console.log("p47", JSON.stringify(v38.toISOString()));
    if (i46 === 0) break L45;
    if ((!v41 > i40 && i40 > i40)) {
      const v48 = C21.make([v41, i46].reduce((s, x) => s + x, v18)).get([n1, v42].length);
      console.log("p49", JSON.stringify(v48));
    } else {
      console.log("p50", JSON.stringify(Object.keys({ a: 3, b: 1 }).join("").repeat(3).padStart(8, "*")));
    }
  }
}
return { n: n1, vars: [n1, v18, v19, v38.toISOString()], calls: [n1, Math.max(v19, n1), { a: 3, b: n1 }.a], o: { a: Object.assign(new RangeError("r"), { code: n1 }).message.length, b: (new Map([["${x}", n1], ["k", v19]]).get("k") ?? 0), m() { return this.a; } } };
