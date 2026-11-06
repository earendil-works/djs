// A subclass's late binding and K's block code naming it: the code goes after the binding
// it reads where no binding it would follow waits for it (D3150; plain-dump fuzzer seed
// 344676, reduced; frozen as regress/s0235-s344676.ts).
let n1 = 0;
class C2 {
  get(x) { return this.value + x + n1; }
  async fetch(x) {}
}
class D3 extends C2 {}
for (let i5 = 0; i5 < 3; i5++) {}
async function* ag13(k) {
  for (let i = 0; i < k; i++) {
    yield await tools.echo(i * 5);
  }
}
for await (const x14 of ag13(1)) {}
async function f24(a25: number): Promise<number> {
  switch (1 % 3) {}
  const v37: Map<string, number> = new Map([[1, [9, n1][0]], ["k", (D3.count++)]]);
}
async function f39(a40: number): Promise<number> {}
