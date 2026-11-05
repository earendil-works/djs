// Found by the plain-dump fuzzer (0211): an async method whose arrow function captures
// `this`, waiting on a tool.
class C {
  constructor(v) { this.value = v; }
  async later(x) {
    const f = () => this.value + x;
    await tools.echo(x);
    return f();
  }
}
console.log(await new C(1).later(2));
const o = { value: 10, async later(x) { const f = () => this.value + x; await tools.echo(x); return f(); } };
console.log(await o.later(3));
return "done";
