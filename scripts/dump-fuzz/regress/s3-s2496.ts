let n1 = 0;
interface I1 { a: number; b?: number }
type T1 = number | string;
function id<T>(x: T): T { return x; }
class C2 {
  constructor(public value: number) {}
  get twice() { return this.value * 2; }
  get v() { return this.value; }
  set v(x) { this.value = x; }
  get(x) { return this.value + x + n1; }
  async fetch(x) { return this.value + (await tools.echo(x)); }
  static make(v) { return new C2(v + 1); }
}
const o3 = new C2((await tools.echo((await tools.echo([3, n1]))[1])));
class C4 {
  constructor(public value: number) {}
  get twice() { return this.value * 2; }
  get v() { return this.value; }
  set v(x) { this.value = x; }
  get(x) { return this.value + x + n1; }
  async fetch(x) { return this.value + (await tools.echo(x)); }
  static make(v) { return new C4(v + 1); }
}
const o5 = new C4([8, (await tools.echo([n1, n1]))[1]][0]);
let v6: { a: number; b: number } = { a: id<number>(n1), b: (await tools.echo([3, 1]))[1] };
let v7: RegExp = /a(b)?/g;
switch (((n1 + n1) + (await tools.echo({ v: n1 })).v) % 3) {
  case (await (async () => (n1))()): {
    console.log("p8", JSON.stringify([new Error("ok").message.length]));
    // falls through
  }
  case 1: {
    for (const x9 of [(await tools.echo(5)), (await tools.echo({ v: 2 })).v]) {
      async function* ag10(k) {
        for (let i = 0; i < k; i++) {
          yield await tools.echo(i * 1);
        }
      }
      for await (const x11 of ag10(2)) {
        let v12 = new Set([[n1, x9].length, n1, x11]);
      }
      let [v13, , v14 = C2.make(x9).get(x9)] = (await tools.echo([8, 4].map((x) => x + x9)));
    }
    const v16 = (await Promise.all((await tools.echo([2, 6])).map(async (x17) => x17 * (await tools.echo((await tools.echo([n1, 4]))[1])))));
    // falls through
  }
}
try {
  let [v18, , v19 = (await tools.echo(n1))] = Object.values(v6).filter((x) => typeof x === "number");
  throw new Error(("b" + 2));
  console.log("p21", JSON.stringify([(true ? n1 : v18), (new Set([v18, v19, 2]).has(1) ? 1 : 0)]));
} catch (e22) {
  console.log("c23", e22.message);
  console.log("p24", JSON.stringify(String((n1 && 2))));
}
async function f25(a26: number): Promise<number> {
  v7.exec("abab xx1 l");
  console.log("p27", JSON.stringify([v7.source, v7.flags, v7.lastIndex]));
  return id<number>(o3.twice);
}
console.log("p28", JSON.stringify((await f25((false ? 3 : n1)))));
{
  using r29 = { [Symbol.dispose]() { console.log("u30"); } };
  let [v31, , v32 = [n1, n1][0]] = Object.values(v6).filter((x) => typeof x === "number").filter((x) => x > { a: 6, b: 9 }.a);
  let v34 = new Uint8Array(new ArrayBuffer(4), 1, 2);
}
return { n: n1, vars: [n1, v6, [v7.source, v7.flags, v7.lastIndex]], calls: [[9, 0][1], (await tools.echo(0)), (7 + 8)], o: { a: (2 || n1), b: (n1 as number) }, last: await tools.echo([1, 2].join("-")) };
