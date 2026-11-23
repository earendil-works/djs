let n1 = 0;
const sym2 = Symbol();
class C3 {
  constructor(v) { this.value = v; }
  get twice() { return this.value * 2; }
  get v() { return this.value; }
  set v(x) { this.value = x; }
  static count = 5;
  static { n1 += 1; }
  get(x) { return this.value + x + n1; }
  async fetch(x) { return this.value + (await tools.echo(x)); }
  async later(x) { const f = () => this.value + x; await tools.echo(x); return f(); }
  static make(v) { return new C3(v + 1); }
}
class D4 extends C3 {
  constructor(v) { super(v * 2); this.extra = v; }
  get(x) { return super.get(x) + this.extra; }
  async fetch(x) { const a = await super.fetch(x); return a + (await tools.echo(this.extra)); }
}
const o5 = new D4((await tools.echo([(4 || n1), [n1, n1][0]]))[1]);
const v6 = new TypeError(new Uint8Array([n1, 0, n1]).join(":"), { cause: (true ? n1 : 7) });
console.log("p7", JSON.stringify([v6.name, v6.message, v6.extra ?? null]));
async function f8(a9) {
  B10: {
    v6.extra = v6.message.length;
    if ([n1, 6].includes(2)) break B10;
    for (let i11 = 0; i11 < 2; i11++) {
      let v12 = new Set([(Math?.max?.(i11, await tools.echo(a9)) ?? 0), (n1 > i11 ? 7 : a9), " ".length]);
      n1 += (await (async () => (i11))());
    }
  }
  let v13 = new Set((await Promise.all([a9, n1].map(async (x14) => (await tools.echo(x14 + (new Set([3, 9, n1]).has(1) ? 1 : 0)))))));
  console.log("p15", JSON.stringify([...v13]));
  for (const x16 of (await tools.echo([2, a9]))) {
    try {
      const v17 = /(?<w>\w)\d/gu;
      await tools.fail({ message: `${"😀"}-${9}:${"line\nbreak"}` });
      n1 -= D4.make([x16, n1].length).get(x16);
    } catch (e18) {
      console.log("c19", e18.message);
    }
    console.log("p20", JSON.stringify([a9, , n1].join("-")));
    console.log("p21", JSON.stringify(/(?<w>\w)\d/gu.source));
  }
  return new Uint8Array([n1, 9, 0]).length;
}
console.log("p22", JSON.stringify((o5.value = (await (async () => (n1))()))));
const { a: v23, b: v24 = ({ a: n1, b: n1 }[sym2] ?? 0) } = { ["a"]: ((s, ...v) => s.length + v.length)`a${1}b${9}`, b: { a: n1, b: 7 }.a, a: (4 + n1), b: (0 + n1) };
for (const k26 in { a: v24, b: n1, [sym2]: n1 }) {
  L27: for (const i28 of [0, 1, 2]) {
    if (o5.get(6) > (v24 ?? n1)) {
      v6.extra = ({ a: 5, b: 9 }[sym2] ?? 0);
      console.log("p29", JSON.stringify([v6.name, v6.message, v6.extra ?? null]));
    }
    try {
      console.log("p30", JSON.stringify(JSON.parse(JSON.stringify({ ...{ a: n1, b: 1 }, b: 1 }))));
      throw new Error(Object.keys({ a: 2, b: v24 }).join(""));
      v6.extra = ({ a: i28, get b() { n1++; return n1; }, set b(v) { n1 = v; } }[sym2] ?? 0);
      console.log("p31", JSON.stringify([v6.name, v6.message, v6.extra ?? null]));
    } catch (e32) {
      console.log("c33", e32.message);
      const v34 = new Map([[/(?<w>\w)\d/gu.source, (true ? n1 : 0)], ["k", ((s, ...v) => s.length + v.length)`a${n1}b${3}`]]);
    } finally {
      (false ? k26 : k26);
      console.log("p35", JSON.stringify([new Date(i28 * 86400000).getUTCMonth(), new Date(i28 * 86400000).getUTCDate()].map((x) => x + (n1, 5))));
    }
    if (i28 === 0) break L27;
    n1 -= /^l/m.lastIndex;
  }
  for (const x36 of [(false ? 2 : v23), [n1, v24].length]) {
    v6.extra = new Map([[" ", n1], ["k", 4]]).size;
  }
}
for (const x37 of [(7 && v23), (v23 || n1)]) {
  for (const k38 in { get a() { return n1 + 1; }, set a(v) { n1 = v; }, b: 0 }) {
    {
      using r39 = { [Symbol.dispose]() { console.log("u40"); } };
      [v23, 4].join("-");
    }
    const v41 = new Set([v6.message.length, (v23 * 2 - v24), (o5.value = 4)]);
  }
}
async function* ag42(k) {
  for (let i = 0; i < k; i++) {
    yield await tools.echo(i * 3);
  }
}
for await (const x43 of ag42(2)) {
  (await tools.echo("a"));
  switch (/./s.lastIndex % 3) {
    case 0: {
      const v44 = new Uint8Array([9, 3, n1]);
      switch ((new Set([6, 7]).has(1) ? 1 : 0) % 3) {
        case 7: {
          v44[((x43 + x43) + (x43 && v24)) % 2] = ((v24 > x43 || true) ? (D4.count++) : [7, 9].reduce((s, x) => s + x, v23));
          v44[x43 % 2] = ((true || v24 > x43) ? (v24 ?? x43) : [3, v23].length);
          break;
        }
      }
      break;
    }
    default: {
      for (let i45 = 0; i45 < 3; i45++) {
        v6.extra = v6.message.length;
        console.log("p46", JSON.stringify([v6.name, v6.message, v6.extra ?? null]));
        const v47 = new Float64Array([[7, x43].length, 0.5]);
        const v48 = /^l/m;
        console.log("p49", JSON.stringify([v48.source, v48.flags, v48.lastIndex]));
      }
    }
  }
}
v6.extra = (((s, ...v) => s.length + v.length)`a${v23}b${2}` * 2 - (3 && v23));
console.log("p50", JSON.stringify([v6.name, v6.message, v6.extra ?? null]));
return { n: n1, vars: [n1, [v6.name, v6.message, v6.extra ?? null], v23, v24], calls: [(new Uint8Array([6, 9, n1])[0] ?? 0), "</script>".length, (6 + 1)], o: JSON.parse(JSON.stringify(new Proxy({ a: 1, b: v24 }, { set(o, k, v) { n1++; o[k] = v; return true; } }))) };
