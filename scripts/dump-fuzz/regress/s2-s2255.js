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
    get(x) { return this.value + x + n1; }
    async fetch(x) { return this.value + (await tools.echo(x)); }
    static make(v) { return new C17(v + 1); }
  }
  const o18 = new C17((await tools.echo((await tools.echo(n1)))));
  if (i10 === 1) continue L9;
  console.log("p19", JSON.stringify([...new Map([["line\nbreak", 1], ["k", i10]]).values()]));
}
const v20 = new Set([[n1, n1][0], 5, [n1, 7].length]);
try {
  String(new Error(" "));
  const f21 = async (a22) => {
    const v23 = new Error([...new Map([["</script>", a22], ["k", n1]]).keys()].join(","));
    console.log("p24", JSON.stringify([v23.name, v23.message, v23.extra ?? null]));
    String(v23);
    return (await new Promise((res) => res((n1 && a22))));
  };
  console.log("p25", JSON.stringify([...g3(2)].reduce((s, x) => s + x, 0)));
} catch (e26) {
  console.log("c27", e26.message);
} finally {
  let w28 = 0;
  while (w28 < 3) {
    w28++;
    for (const k29 in (await tools.echo({ a: n1, b: 0 }))) {
      console.log("p30", JSON.stringify((typeof (9 + 5))));
    }
    switch (new Uint8Array(new ArrayBuffer(4), 1, 2).length % 3) {
      case 0: {
        console.log("p31", JSON.stringify(Object.values({ a: n1, b: n1 }).filter((x) => typeof x === "number").map((x) => x + (1 || n1))));
        "ok";
        // falls through
      }
      case 8: {
        const [v32 = (await tools.echo(n1)), ...v34] = [[n1, n1].length, (n1 ?? n1)];
        console.log("p35", JSON.stringify(((new Map(Object.entries({ a: 6, b: n1 })).get("k") ?? 0) * 2 - (v20.has(1) ? 1 : 0))));
        break;
      }
    }
    class C36 {
      constructor(v) { this.value = v; }
      get twice() { return this.value * 2; }
      get v() { return this.value; }
      set v(x) { this.value = x; }
      get(x) { return this.value + x + n1; }
      async fetch(x) { return this.value + (await tools.echo(x)); }
      static make(v) { return new C36(v + 1); }
    }
    const o37 = new C36(Uint8Array.from([8, 9]).join(":").length);
  }
}
console.log("p38", JSON.stringify(`${String(n1)}-${[n1, n1].length}:${JSON.stringify({ a: 3, b: 2 })}`));
const f39 = async (a40) => {
  v20.add({ ...{ a: n1, b: n1 }, b: a40 }.a);
  console.log("p41", JSON.stringify([...v20]));
  console.log("p42", JSON.stringify([new Date(Date.UTC(2024, n1 % 12, n1 + 1)).getUTCMonth(), new Date(Date.UTC(2024, n1 % 12, n1 + 1)).getUTCDate()]));
  return [a40, 8].filter((x) => x > 1).length;
};
if ((2 === 4 && "s1" !== "")) {
  v20.add(2);
  `${"x"}-${n1}:${"x"}`;
  switch (n1 % 3) {
    case n1: {
      n1 += ([3, 1].length * 2 - 2);
      "a";
      break;
    }
    case ({ a: n1, b: n1 }[sym2] ?? 0): {
      function* g43(k) {
        for (let i = 0; i < k; i++) yield i + n1;
      }
      break;
    }
  }
}
return { n: n1, vars: [n1, [...v20]], calls: [[n1, n1].reduce((s, x) => s + x, n1), (false ? n1 : 5), (n1 + n1)], o: { ...{ a: n1, b: n1 }, b: 7 } };
