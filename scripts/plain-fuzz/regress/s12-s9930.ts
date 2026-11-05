let n1 = 0;
const sym2 = Symbol.for("shared");
interface I2 { a: number; b?: number }
type T2 = number | string;
function id<T>(x: T): T { return x; }
(await tools.echo("quo\"te"));
let v3: { a: number; b: number } = { a: (2 && 3), b: (await tools.echo(n1)) };
const [v4 = (await new Promise((res) => res(5))), ...v6] = [(await tools.echo(n1)), (n1 + 6)];
let n7 = (5, v4);
const f8 = (x) => { n7++; return n7 + x; };
console.log("p9", JSON.stringify(f8((await (async () => (v4))()))));
console.log("p10", JSON.stringify(`${String(Object.assign(new RangeError("r"), { code: 5 }))}-${(v6.sum ?? 0)}:${[...new Map(Object.entries({ a: 7, b: n7 })).keys()].join(",")}`));
{
  await using r11 = { async [Symbol.asyncDispose]() { console.log("u12", await tools.echo("bye")); } };
  switch ((v6.sum ?? 0) % 3) {
    case v6.reduce((s, x) => s + x, 6): {
      const v13: { a: number; b: number } = v3;
      console.log("p14", JSON.stringify(v13));
      break;
    }
    case (7 ?? n1): {
      switch ((f8(n1) + (await tools.echo({ v: 8 })).v) % 3) {
        case (await tools.echo([v4, 1]))[1]: {
          let v15: Uint8Array | Float64Array = new Uint8Array([id<number>(1), (await (async () => (1))()), (new Date(6 * 86400000).getTime() / 86400000)]);
          break;
        }
        default: {
          if (4 === v4) break;
        }
      }
      for (const x16 of [(await tools.echo(v4)), n1]) {
        new Uint8Array([1, n7, n1]).join(":");
        console.log("p17", JSON.stringify([(0, n1), new Map(Object.entries(v3)).size]));
        console.log("p18", JSON.stringify((5 + v3.a)));
      }
      break;
    }
    case 2: {
      if ((Math?.max?.(n1, await tools.echo(n1)) ?? 0) === v6.reduce((s, x) => s + x, n1)) {
        console.log("p19", JSON.stringify((await tools.echo((await tools.echo({ a: n1, b: 2 }))))));
        n1 = ((n7 as number) && (false ? v4 : n1));
        let [v20, , v21 = (0 as number)] = v6.map((x) => x + (v6.sum ?? 0));
      }
      // falls through
    }
  }
}
let v23: bigint = (2n ** 48n + 2n);
if ((await tools.echo({ v: 9 })).v > (await tools.echo(4))) {
  console.log("p26", JSON.stringify((await Promise.all((await Promise.all(v6.map(async (x25) => x25 * (await tools.echo(f8(x25)))))).map(async (x24) => (await tools.echo(x24 + new Error("x").message.length)))))));
  console.log("p27", JSON.stringify(new Proxy({ a: (false ? n7 : v4), b: v4 }, { get: (o, k) => (k === "b" ? n1 : o[k]) })));
} else {
  try {
    try {
      B28: {
        const v29 = (2n ** 72n + 0n);
        console.log("p30", JSON.stringify([...(await tools.echo("b"))].reverse().join("")));
        if ([0, 8].includes(v4)) break B28;
        console.log("p31", JSON.stringify((await tools.echo((await new Promise((res) => res(2)))))));
      }
    } finally {
      const { a: v32, b: v33 = (n1, 9) } = new Proxy({ a: n7, b: Math.max(n1, 7) }, { get: (o, k) => (k === "b" ? n1 : o[k]) });
      let w35 = 0;
      while (w35 < 3) {
        w35++;
        console.log("p36", JSON.stringify([v32, v33, 6].join("-")));
        let v37: RegExp = new RegExp("a", "g");
        console.log("p38", JSON.stringify([v37.source, v37.flags, v37.lastIndex]));
      }
    }
  } catch (e) {
    console.log("outer", e.message);
  }
}
return { n: n1, vars: [n1, v3, v4, v6, n7, String(v23)], calls: [((s, ...v) => s.length + v.length)`a${v4}b${n1}`, (v3[sym2] ?? 0), ((s, ...v) => s.length + v.length)`a${n1}b${0}`], o: v3, last: await tools.echo(("</script>" + n7)) };
