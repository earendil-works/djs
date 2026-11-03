// Every operand role a codemode await can have below it, each in a function of its own,
// all waiting at once.
// roles.dump.js, written by hand, is this program at idle point 1: every function waits
// at its first await, main on their Promise.all.
class Thing {
  constructor(...a) {
    this.a = a;
  }
}
class Base {
  m(x) {
    return "base " + x;
  }
}
class Child extends Base {
  async m(x) {
    return super[await later("m")](x);
  }
}
const o = { k: 1, f(a, b, c) { return [this.k, a, b, c].join(); } };
async function call3() { return o.f(1, await later(2), 3); }
async function spread() { return o.f(...[1], await later(2), 3); }
async function plain() { return Math.max(1, await later(5)); }
async function newer() { return new Thing(1, await later(2)).a.join(); }
async function newSpread() { return new Thing(...[1], await later(2)).a.join(); }
async function member() { const t = { x: 9 }; return t[await later("x")]; }
async function key() { const k = "z"; return JSON.stringify({ a: 1, [k]: await later(2) }); }
async function value() { return JSON.stringify({ a: 1, c: await later(3) }); }
async function elements() { return [1, 2, await later(3)].join(); }
async function hole() { return [1, , await later(3)].length; }
async function left() { const a = 5; return a * (await later(6)); }
async function prop() { const t = {}; t.p = await later(1); return JSON.stringify(t); }
async function index() { const t = {}, k = "q"; t[k] = await later(2); return JSON.stringify(t); }
async function compound() { let n = 1; n += await later(4); return n; }
async function compoundProp() { const t = { r: 1 }; t.r += await later(2); return t.r; }
async function template() { return `a${1}b${await later(2)}c`; }
async function main() {
  const rs = await Promise.all([
    call3(), spread(), plain(), newer(), newSpread(), member(), key(), value(), elements(),
    hole(), left(), prop(), index(), compound(), compoundProp(), template(), new Child().m(7),
  ]);
  for (const r of rs) print(r);
}
main();
