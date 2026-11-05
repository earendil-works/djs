let n1 = 0;
const sym2 = Symbol("k");
interface I2 { a: number; b?: number }
type T2 = number | string;
function id<T>(x: T): T { return x; }
enum E3 { A, B = 9, C }
for (const x4 of [n1, (4 ?? n1)]) {
  let v5: RegExp = new RegExp("[ab]", "g");
  console.log("p6", JSON.stringify([v5.source, v5.flags, v5.lastIndex]));
  let n7 = (await tools.echo(n1));
  const f8 = (x) => (n7 += x);
  if ((x4 as number) > [n1, 4].length) {
    let [v9, , v10 = (await (async () => (n1))())] = (await Promise.all([x4, ...[n7, n1]].map(async (x12) => (await tools.echo(x12 + E3.B)))));
  }
}
String(new Error("😀"));
const v13: RegExp = /./s;
let v14: Error = new TypeError((await tools.echo("")), { cause: 8 });
console.log("p15", JSON.stringify([v14.name, v14.message, v14.extra ?? null]));
if ((!7 > n1 && (true && false))) {
  (typeof 7);
}
let v16 = (await tools.echo(("</script>" + 2)));
let v17 = new Uint8Array([Math.max(n1, n1), (await tools.echo(8)), (await tools.echo(n1))]);
console.log("p18", JSON.stringify(Array.from(v17)));
async function* ag19(k) {
  for (let i = 0; i < k; i++) {
    yield await tools.echo(i * 3);
  }
}
for await (const x20 of ag19(1)) {
  if ((n1 + 0) > (n1 + n1)) {
    v13.exec("abab xx1 l");
    console.log("p21", JSON.stringify([v13.source, v13.flags, v13.lastIndex]));
    let v22 = new Set([3, 3].map((x) => x + 4));
  }
  try {
    E3[E3.A];
    await tools.fail({ message: "a".repeat(3).padStart(8, "*") });
  } catch (e23) {
    console.log("c24", e23.message);
  } finally {
    async function* ag25(k) {
      for (let i = 0; i < k; i++) {
        yield await tools.echo(i * 2);
      }
    }
    for await (const x26 of ag25(2)) {
      let v27 = v16;
      let [v28, , v29 = (await new Promise((res) => res(x20)))] = (await Promise.all([8, ...[2, x20]].map(async (x31) => (await tools.echo(x31 + 5)))));
    }
  }
}
return { n: n1, vars: [n1, [v13.source, v13.flags, v13.lastIndex], [v14.name, v14.message, v14.extra ?? null], v16, Array.from(v17)], calls: [5, n1, n1], o: { a: (n1 as number), b: n1, get g() { return n1 * 2; } } };
