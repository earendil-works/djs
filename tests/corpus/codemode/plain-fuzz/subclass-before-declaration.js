// difftest: plain-xfail a subclass in K is published (`Object.assign($dump.sN, { D3 })`) before its declaration and into a factory the dump never makes: the load throws
// Found by the plain-dump fuzzer (0211): a subclass with an async method calling `super`,
// named by an async function that has not run yet, dumped inside a `new Map([...])`.
let n1 = 0;
class C2 {
  static make(v) { return new C2(v + 1); }
}
class D3 extends C2 {
  async fetch(x) { const a = await super.fetch(x); return a + (await tools.echo(this.extra)); }
}
async function f5(a6) {
  switch ((await tools.echo({ v: (await new Promise((res) => res(a6))) })).v % 3) {
    case 0: {
      {
        const { a: v9, b: v10 = D3.make(n1).get(a6) } = { a: (a6 + 9), get b() { n1++; return n1; }, set b(v) { n1 = v; } };
      }
    }
  }
}
let w12 = 0;
do {
  w12++;
  class C13 {
  }
  let w15 = 0;
  while (w15 < 1) {
    w15++;
    B16: {
    }
  }
} while (w12 < 2);
let v19 = new Map([[(await tools.echo("</script>")), (3 && 3)], ["k", (await f5(n1))]]);
for (let i21 = 0; i21 < 1; i21++) {
}
const f22 = async (a23) => {
};
