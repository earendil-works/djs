// djs-dump 2: 18 activations, 17 pending host calls.
// at idle point 1: every function waits at its first await, main on their Promise.all.
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

const L = (id, v) => $dump.pending(id, "later", [v]);
const r = (f, at, awaiting, more) => $dump.resume(f, { at, awaiting, ...more });
$dump.t1 = { x: 9 };
$dump.t2 = {};
$dump.t3 = {};
$dump.t4 = { r: 1 };
$dump.child = new Child();
$dump.all = [
  r(call3, "await#1", L(1, 2), { operands: [["call.callee", o.f], ["call.this", o], ["call.arg[0]", 1]] }),
  r(spread, "await#1", L(2, 2), { operands: [["call.callee", o.f], ["call.this", o], ["call.args", [1]]] }),
  r(plain, "await#1", L(3, 5), { operands: [["call.callee", Math.max], ["call.this", Math], ["call.arg[0]", 1]] }),
  r(newer, "await#1", L(4, 2), { operands: [["new.callee", Thing], ["new.arg[0]", 1]] }),
  r(newSpread, "await#1", L(5, 2), { operands: [["new.callee", Thing], ["new.args", [1]]] }),
  r(member, "await#1", L(6, "x"), { bindings: { t: $dump.t1 }, operands: [["member.object", $dump.t1]] }),
  r(key, "await#1", L(7, 2), {
    bindings: { k: "z" },
    operands: [["call.callee", JSON.stringify], ["call.this", JSON], ["object.literal", { a: 1 }], ["object.key", "z"]],
  }),
  r(value, "await#1", L(8, 3), {
    operands: [["call.callee", JSON.stringify], ["call.this", JSON], ["object.literal", { a: 1 }]],
  }),
  r(elements, "await#1", L(9, 3), { operands: [["array.element[0]", 1], ["array.element[1]", 2]] }),
  r(hole, "await#1", L(10, 3), { operands: [["array.literal", [1, ,]]] }),
  r(left, "await#1", L(11, 6), { bindings: { a: 5 }, operands: [["binary.left", 5]] }),
  r(prop, "await#1", L(12, 1), { bindings: { t: $dump.t2 }, operands: [["assign.object", $dump.t2]] }),
  r(index, "await#1", L(13, 2), {
    bindings: { t: $dump.t3, k: "q" },
    operands: [["assign.object", $dump.t3], ["assign.key", "q"]],
  }),
  r(compound, "await#1", L(14, 4), { bindings: { n: 1 }, operands: [["assign.old", 1]] }),
  r(compoundProp, "await#1", L(15, 2), {
    bindings: { t: $dump.t4 },
    operands: [["assign.object", $dump.t4], ["assign.old", 1]],
  }),
  r(template, "await#1", L(16, 2), {
    operands: [["template.part[0]", "a"], ["template.part[1]", "1"], ["template.part[2]", "b"]],
  }),
  r(Child.prototype.m, "await#1", L(17, "m"), {
    this: $dump.child,
    args: [7],
    bindings: { x: 7 },
    operands: [["super.this", $dump.child], ["super.base", Base.prototype]],
  }),
];
$dump.handle("main", $dump.resume(main, {
  at: "await#1",
  awaiting: Promise.all($dump.all),
  bindings: { rs: $dump.TDZ },
}));
