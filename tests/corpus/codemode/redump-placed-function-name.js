// Placed code K writes after a late binding it reads must not move behind one that
// reads what it makes: `Object.assign($dump.sN, { ag7: async function* ag7() {…} })`
// names ag7 itself, and `var ag7 = $dump.sN.ag7` reads it (0215; plain-dump fuzzer seed 832780).
let n1 = 0;
class C2 {
  constructor(v) { this.value = v; }
  get twice() { return this.value * 2; }
  get v() { return this.value; }
  set v(x) { this.value = x; }
  static count = n1;
  static { n1 += 1; }
  get(x) { return this.value + x + n1; }
  async fetch(x) { return this.value + (await tools.echo(x)); }
  async later(x) { const f = () => this.value + x; await tools.echo(x); return f(); }
  static make(v) { return new C2(v + 1); }
}
class D3 extends C2 {
  constructor(v) { super(v * 2); this.extra = v; }
  get(x) { return super.get(x) + this.extra; }
  async fetch(x) { const a = await super.fetch(x); return a + (await tools.echo(this.extra)); }
}
const o4 = new D3([4, ...[n1, n1]].length);
try {
  await tools.fail({ message: (await tools.echo("ok")) });
} catch (e5) {
  console.log("c6", e5.message);
}
async function* ag7(k) {
  for (let i = 0; i < k; i++) {
    yield await tools.echo(i * 5);
  }
}
for await (const x8 of ag7(2)) {
  B9: {
    if (("a" in { a: 2, b: n1 } && "a" !== "</script>")) {
      ;
      console.log("p13", JSON.stringify([...new Set([3, x8, n1])].join(",").length));
    } else {
      let [v14, , v15 = (await tools.echo(x8))] = [x8, n1, n1].filter((x) => x > (9 || 8));
      let v17 = (await tools.echo([v15, n1])).map((x) => x + (v14 && n1));
    }
    if ([n1, x8].includes(5)) break B9;
    for (let i18 = 0; i18 < 2; i18++) {
      console.log("p19", JSON.stringify((await tools.echo([-0, NaN, Infinity, n1 / 0].map((x) => (Object.is(x, -0) ? "-0" : String(x))).join(",")))));
    }
  }
}
console.log("p20", JSON.stringify((new Uint8Array([n1, 8, 8])[0] ?? 0)));
n1 += new Date(Date.UTC(2024, 4 % 12, n1 + 1)).getUTCDate();
for (const k21 in Object.freeze({ a: n1, b: n1 })) {
  n1 = Number((2n ** 73n + 7n) % 1000n);
}
let w22 = 0;
do {
  w22++;
  class C23 {
    constructor(v) { this.value = v; }
    get twice() { return this.value * 2; }
    get v() { return this.value; }
    set v(x) { this.value = x; }
    static count = 6;
    static { n1 += 1; }
    get(x) { return this.value + x + n1; }
    async fetch(x) { return this.value + (await tools.echo(x)); }
    async later(x) { const f = () => this.value + x; await tools.echo(x); return f(); }
    static make(v) { return new C23(v + 1); }
  }
  const o24 = new C23((await tools.echo([new C2(n1).twice, [n1, 2].length]))[1]);
  if ((await tools.echo(n1)) > (n1 && n1)) {
    for (let i25 = 0; i25 < 3; i25++) {
      const v26 = new Uint8Array(new ArrayBuffer(4), 1, 2);
    }
  } else {
    try {
      try {
        n1 = (await tools.echo(((s, ...v) => s.length + v.length)`a${n1}b${5}`));
      } finally {
        let v27 = new RegExp("[ab]", "g");
        console.log("p28", JSON.stringify(new TypeError("ok", { cause: n1 }).message.length));
      }
    } catch (e) {
      console.log("outer", e.message);
    }
  }
  console.log("p29", JSON.stringify((await tools.echo(`${"b"}-${3}:${""}`))));
} while (w22 < 3);
let v30 = { a: 9, get b() { n1++; return n1; }, set b(v) { n1 = v; } };
console.log("p31", JSON.stringify(v30));
return { n: n1, vars: [n1, v30], calls: [(await new Promise((res) => res(6))), (true ? 2 : 6), [n1, n1].length], o: { a: (await new Promise((res) => res(n1))), b: 0, m() { return this.a; } }, last: await tools.echo((await tools.echo("x"))) };
