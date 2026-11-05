// Arrows that read a waiting function's `this`, `arguments` or `new.target`: its
// continuation is called with that `this`, and reads the saved arguments object as
// `$djs_arguments`, the arrows too (0215; pjz's class method with `items.map(x =>
// this.f(x))` around an await was refused, an arrow's `arguments` read K's own).
class C {
  constructor(v) { this.value = v; }
  scale(x) { return x * this.value; }
  async later(x) { const f = () => this.value + x; await tools.echo(x); return f(); }
  async all(items) {
    const out = items.map(x => this.scale(x));
    await tools.echo(out.length);
    return out.concat(items.map(x => this.scale(x + 1)));
  }
}
const o = { k: 5, async m(x) { const g = () => this.k * x; await tools.echo(x); return g(); } };
async function g() { const f = () => arguments.length; await tools.echo(1); return f(); }
async function h() {
  const args = arguments, f = () => arguments[1], nt = () => new.target;
  await tools.echo(2);
  return [f(), args === arguments, String(nt()), arguments.length];
}
function* gen() { const f = () => this.k + arguments[0]; yield 1; return f(); }
console.log(await new C(1).later(2));
console.log((await new C(2).all([1, 2])).join());
console.log(await o.m(3));
console.log(await g(1, 2));
console.log((await h(1, 7, 9)).join());
const it = gen.call({ k: 10 }, 4); it.next(); await tools.echo(3);
console.log(it.next().value);
