// Found by the plain-dump fuzzer (0207, pjz, seed 10014): a continuation's binding holds a
// function with types; the dump writes it with the types blanked (several spaces), and a
// re-dump reading the value back must read past them as the writer read past the types.
let n1 = 0;
let n3 = /^l/m.lastIndex;
const f4 = (x) => { n3++; return n3 + x; };
let w6 = 0;
while (w6 < 1) {
  w6++;
  async function* ag23(k) { for (let i = 0; i < k; i++) {} }
  try {} catch (e32) {}
}
for (let i34 = 0; i34 < 2; i34++) {
  let [v35, , v36 = (Math?.max?.(n1, await tools.echo(n3)) ?? 0)] = [Object.assign(new RangeError("r"), { code: n3 }).message.length];
  async function f38(a39: number): Promise<number> {
    for (const x40 of [(v36 && n1), 4].entries().map(([k, x]) => k + x)) {
      switch ((f4(n3) + a39) % 3) { case 0: {} }
    }
  }
  console.log("p53", JSON.stringify((await f38((5 ?? i34)))));
}
