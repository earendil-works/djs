let n1 = 0;
const sym2 = Symbol("k");
let n3 = (6, 7);
const f4 = (x) => { n3++; return n3 + x; };
const v5 = (await tools.echo({ a: n3, b: n3 }));
console.log("p6", JSON.stringify(v5));
console.log("p7", JSON.stringify(Object.values({ a: n1, get b() { n1++; return n1; }, set b(v) { n1 = v; } }).filter((x) => typeof x === "number")));
let n8 = (2, n1);
function f9(x) { return n8 * 2 + x; }
console.log("p10", JSON.stringify(f9(8)));
n3 = 9;
(await tools.echo("😀"));
for (const x11 of [(await tools.echo(5)), (n1 > n8 ? n3 : n8)].entries().map(([k, x]) => k + x)) {
  B12: {
    for (let i13 = 0; i13 < 1; i13++) {
      v5.b = ((2 ?? n1) + (await tools.echo(n1)));
      console.log("p14", JSON.stringify((await tools.echo({ a: 9, b: 6 }))));
    }
    let v15 = (Math?.max?.(6, await tools.echo(9)) ?? 0);
    console.log("p16", JSON.stringify(v15));
    if ((x11 > n3 || true)) break B12;
    let v17 = new Date(Date.UTC(2024, (7 + 1) % 12, (await tools.echo(6)) + 1));
    console.log("p18", JSON.stringify(v17.toISOString()));
  }
  async function f19(a20) {
    for (const k21 in JSON.parse(JSON.stringify(v5))) {
      switch ((await tools.echo((await tools.echo({ v: a20 })).v)) % 3) {
        case 0: {
          n1 += (await tools.echo(f9(a20)));
          break;
        }
        case 1: {
          console.log("p22", JSON.stringify([[6, 3][1], (await tools.echo({ v: n8 })).v]));
          v5[(typeof 1) || "a"] = ((s, ...v) => s.length + v.length)`a${/x+/y.lastIndex}b${(await tools.echo(3))}`;
          // falls through
        }
        case 2: {
          let v23 = new Map([[(await tools.echo(k21)), n3], ["k", 2]]);
          // falls through
        }
        default: {
          v5.b = ((Math?.max?.(2, await tools.echo(3)) ?? 0) || (v5[sym2] ?? 0));
        }
      }
    }
    if (String(/a(b)?/g.exec("xabab x1 l")) !== "quo\"te") {
      if (new Date(Date.UTC(2024, n1 % 12, n8 + 1)).getUTCDate() === (5 + a20)) {
        v5.b = (await tools.echo((await tools.echo(4))));
        console.log("p24", JSON.stringify("${x}"));
        let v25 = new Date(f9(9) * 86400000);
        console.log("p26", JSON.stringify(v25.toISOString()));
      }
      try {
        try {
          throw new Error([4, n1].join("-"));
          let v27 = (2n ** 59n + 1n);
        } finally {
          console.log("p28", JSON.stringify([(n8 + 1), (false ? 7 : a20), a20]));
        }
      } catch (e) {
        console.log("outer", e.message);
      }
      for (const x29 of [((s, ...v) => s.length + v.length)`a${n1}b${5}`, (0, n1)].entries().map(([k, x]) => k + x)) {
        console.log("p30", JSON.stringify([x29, ...[x11, n8]].map((x) => x + (a20, 6))));
        n8 = 5;
        v5.b = ((s, ...v) => s.length + v.length)`a${(n8 && 7)}b${Object.assign(new RangeError("r"), { code: n3 }).message.length}`;
      }
    } else {
      for (let i31 = 0; i31 < 1; i31++) {
        console.log("p32", JSON.stringify(((n3 > x11 && true) ? i31 : ({ a: i31, b: a20 }[sym2] ?? 0))));
        if ("s1" !== "quo\"te") continue;
        console.log("p33", JSON.stringify(4));
      }
      try {
        const v34 = /./s;
      } catch (e35) {
        console.log("c36", e35.message);
      } finally {
        console.log("p37", JSON.stringify(Object.defineProperty([1, [a20, n1].length], 2, { get: () => n3, enumerable: true, configurable: true })));
        console.log("p38", JSON.stringify(v5));
      }
      console.log("p39", JSON.stringify(Object.values({ a: a20, b: a20, [sym2]: 8 }).filter((x) => typeof x === "number")));
    }
    for (const x40 of [(a20 && n8), (x11 + n1)].entries().map(([k, x]) => k + x)) {
      n3 = n1;
      try {
      } catch (e41) {
        console.log("c42", e41.message);
      } finally {
        n1 += Math.max(v5.a, (9 && 0));
        console.log("p43", JSON.stringify((x40, new Uint8Array([4, x40, 5]).byteOffset)));
      }
      try {
        throw new Error("b");
      } catch (e44) {
        console.log("c45", e44.message);
      } finally {
        "ok";
      }
    }
    return [(new Map([["", n3], ["k", x11]]).get("k") ?? 0), new Error("a").message.length][1];
  }
  console.log("p46", JSON.stringify(f9((await tools.echo([n3, x11]))[1])));
  n3 = n8;
}
delete v5.c;
return { n: n1, vars: [n1, n3, v5, n8], calls: [(new Set([n1, 6, n8]).has(1) ? 1 : 0), (n8, 8), (await tools.echo(n1))], o: (await tools.echo((await tools.echo({ a: n8, b: n3 })))), last: await tools.echo(("quo\"te" + n8)) };
