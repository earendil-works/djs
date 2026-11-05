let n1 = 0;
const sym2 = Symbol.for("shared");
function* g3(k) {
  for (let i = 0; i < k; i++) yield i + n1;
}
let w4 = 0;
while (w4 < 1) {
  w4++;
  console.log("p5", JSON.stringify((n1 === n1 ? [2, 8][0] : (n1 || 6))));
  {
    using r6 = { [Symbol.dispose]() { console.log("u7"); } };
    for (let i8 = 0; i8 < 1; i8++) {
      new Uint8Array([9, i8, 1]).join(":");
    }
  }
}
L9: for (const i10 of [0, 1, 2]) {
  try {
    String(new Error("").cause);
    async function* ag11(k) {
      for (let i = 0; i < k; i++) {
        yield await tools.echo(i * 5);
      }
    }
    for await (const x12 of ag11(2)) {
      console.log("p13", JSON.stringify((await tools.echo([x12, x12])).map((x) => x + n1)));
      const v14 = [{ a: i10, b: i10 }.a, (await tools.echo({ v: i10 })).v][0];
    }
  } catch (e15) {
    console.log("c16", e15.message);
  }
  class C17 {
    constructor(v) { this.value = v; }
    get twice() { return this.value * 2; }
    get v() { return this.value; }
    set v(x) { this.value = x; }
    static count = i10;
    get(x) { return this.value + x + n1; }
    async fetch(x) { return this.value + (await tools.echo(x)); }
    static make(v) { return new C17(v + 1); }
  }
  const o18 = new C17((await tools.echo((false ? i10 : n1))));
  if (i10 === 2) break L9;
}
async function* ag19(k) {
  for (let i = 0; i < k; i++) {
    yield await tools.echo(i * 1);
  }
}
for await (const x20 of ag19(2)) {
  for (const x21 of [[x20, n1][0], 5]) {
    console.log("p23", JSON.stringify((await Promise.all([7, 7].map((x) => x + x21).map(async (x22) => x22 * (await tools.echo(6)))))));
    console.log("p24", JSON.stringify(JSON.parse(JSON.stringify({ a: x20, b: 2 }))));
    function* g25(k) {
      for (let i = 0; i < k; i++) yield i + n1;
    }
  }
}
[...""].reverse().join("");
console.log("p26", JSON.stringify(((false || true) ? (n1 || 4) : [n1, n1].length)));
const v27 = [4, (n1 && n1), (n1 + n1)];
class C28 {
  constructor(v) { this.value = v; }
  get twice() { return this.value * 2; }
  get v() { return this.value; }
  set v(x) { this.value = x; }
  static count = n1;
  get(x) { return this.value + x + n1; }
  async fetch(x) { return this.value + (await tools.echo(x)); }
  static make(v) { return new C28(v + 1); }
}
const o29 = new C28(((1 || n1) && (1 || n1)));
return { n: n1, vars: [n1, v27], calls: [2, (n1 && n1), n1], o: { ...{ ...{ a: n1, b: n1 }, b: n1 }, b: 9 } };
