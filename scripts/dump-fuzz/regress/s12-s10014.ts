let n1 = 0;
const sym2 = Symbol();
interface I2 { a: number; b?: number }
type T2 = number | string;
function id<T>(x: T): T { return x; }
let n3 = /^l/m.lastIndex;
const f4 = (x) => { n3++; return n3 + x; };
console.log("p5", JSON.stringify(f4([5, n1][1])));
let w6 = 0;
while (w6 < 1) {
  w6++;
  for (const x7 of (await tools.echo([6, n1]))) {
    function* g8(k) {
      for (let i = 0; i < k; i++) yield i + x7;
    }
    console.log("p9", JSON.stringify((await tools.echo({ a: n1, b: 8 }.b))));
    console.log("p10", JSON.stringify(BigInt(x7 | 0).toString(16)));
  }
}
const f11 = async (a12: number): Promise<number> => {
  n1 -= n3!;
  return (id<number>(n1), 2);
};
let w13 = 0;
while (w13 < 1) {
  w13++;
  if (7 > (await tools.echo(n3))) {
    function* g14(k) {
      for (let i = 0; i < k; i++) yield i + n3;
    }
  } else {
    `${"a\\b"}-${n3}:${"üñí"}`;
  }
  let [v15, , v16 = { a: 7, b: n1 }.b] = (await Promise.all([/./s.lastIndex].map(async (x18) => (await tools.echo(x18 + 5)))));
}
;
for (const k22 in new Proxy({ a: n3, b: 3 }, {})) {
  async function* ag23(k) {
    for (let i = 0; i < k; i++) {
      yield await tools.echo(i * 2);
    }
  }
  for await (const x24 of ag23(2)) {
    const { a: v25, ...v27 } = await tools.echo({ a: (await (async () => (n1))()), b: ((s, ...v) => s.length + v.length)`a${x24}b${x24}` });
  }
  try {
    B28: {
      let [v29, , v30 = (await tools.echo(9))] = [(n3 as number), { a: 3, b: n3 }.b];
      if ("a" in { a: v29, b: 4 }) break B28;
      n3 = (await tools.echo((0 || v30)));
    }
    await tools.fail({ message: ("</script>" + 0) });
  } catch (e32) {
    console.log("c33", e32.message);
  }
}
for (let i34 = 0; i34 < 2; i34++) {
  let [v35, , v36 = (Math?.max?.(n1, await tools.echo(n3)) ?? 0)] = [Object.assign(new RangeError("r"), { code: n3 }).message.length];
  async function f38(a39: number): Promise<number> {
    for (const x40 of [(v36 && n1), 4].entries().map(([k, x]) => k + x)) {
      switch ((f4(n3) + a39) % 3) {
        case 0: {
          ;
          let [v44, , v45 = id<number>(1)] = Object.defineProperty([(n3 ?? v35), new Set([a39, 0]).size], "sum", { get() { return this[0] + this[1] + v35; } });
          break;
        }
        case "</script>".length: {
          const v47: bigint = BigInt(f4(5) | 0);
          console.log("p48", JSON.stringify(String(v47)));
          let v49 = new Set([(v36 || n3), (await tools.echo([x40, v35]))[1], (7 + v35)]);
          break;
        }
        default: {
          const v50 = (await tools.echo(4));
          console.log("p51", JSON.stringify(v50));
        }
      }
    }
    let v52 = { ["a"]: (n1 * 2 - v35), b: n3!, a: (7 + n3), b: 5 };
    return v36!;
  }
  console.log("p53", JSON.stringify((await f38((5 ?? i34)))));
}
console.log("p54", JSON.stringify([(n1 + 8), new Error("").message.length][0]));
return { n: n1, vars: [n1, n3], calls: [(1 + 1), (new Date(Date.UTC(2024, n1 % 12, n3 + 1)).getTime() / 86400000), ({ a: n1, b: n1 }[sym2] ?? 0)], o: (new Proxy({ a: 3, b: 8 }, { set(o, k, v) { n1++; o[k] = v; return true; } }) satisfies object) };
