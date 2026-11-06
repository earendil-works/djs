// One member line of K's block code fills f33, whose late binding waits for it, and reads
// the later late D39: the late bindings go in the order what they read needs, not as
// declared (D3150; plain-dump fuzzer seed 208872, reduced; frozen as regress/s0235-s208872.ts).
let n1 = 0;
function id<T>(x: T): T {}
switch (1 % 3) {}
{}
if (1 === 7) {}
const f33 = async (a34: number): Promise<number> => {
  return (1 ? 1 : (await tools.echo({ v: n1 })).v);
};
class C38 {
  static make(v) { return new C38(v + 1); }
}
class D39 extends C38 {
  async fetch(x) { const a = await super.fetch1; return a + (await tools.echo1); }
}
const o40 = new D39(n1);
async function f41(a42: number): Promise<number> {
  try {} catch (e47) {
    switch (1`a${n1}b${1}` % 3) {
      case 0: {
        const v50: Set<number> = new Set([1, new D39(a42).twice, 1]);
      }
    }
  }
}
return { n: n1, vars: [], calls: [], o: {}, last: await tools.echo(String(n1)) };
