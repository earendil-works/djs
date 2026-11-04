// difftest: plain-xfail two classes placed in one continuation block get the same temporary ($djs_c1_0_0): the dump does not load
// Found by the plain-dump fuzzer (0208): a class declared in a switch's block next to
// one in the body, both naming variables of the codemode body and themselves.
let n1 = 0;
class C2 {
  async fetch(x) { return this.value + (await tools.echo(x)); }
  static make(v) { return new C2(v + 1); }
}
switch (1 % 3) {
  default: {}
  async function f14(a15) {}
  class C29 {
    async fetch(x) { return this.value + (await tools.echo(x)); }
    static make(v) { return new C29(v + 1); }
  }
  n1 += await tools.echo(1);
}
return n1;
