// K's block code `Object.assign($dump.sN, { ag42: async function* ag42() {…}, D4 })` reads
// the late D4 and names ag42, whose late binding waits for it: after D4, before ag42 (D3150;
// plain-dump fuzzer seed 342347, reduced; frozen as regress/s0235-s342347.js).
let n1 = 0;
class C3 {
  get(x) { return this.value + x + n1; }
}
class D4 extends C3 {}
const o5 = new D4((await tools.echo([]))[1]);
async function f8(a9) {
  B10: {}
  for (const x16 of 1) {
    try {
      n1 -= D4.make1.get1;
    } catch (e18) {}
  }
}
for (const k26 in {}) {}
async function* ag42(k) {}
