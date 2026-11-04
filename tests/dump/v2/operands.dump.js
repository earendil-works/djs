"djs-dump 2";
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
  r(call3, "await#1", L(1, 2), { slots: { "call.callee@32": o.f, "call.this@32": o, "call.arg@36": 1 } }),
  r(spread, "await#1", L(2, 2), { slots: { "call.callee@33": o.f, "call.this@33": o, "call.args@36": [1] } }),
  r(plain, "await#1", L(3, 5), { slots: { "call.callee@32": Math.max, "call.this@32": Math, "call.arg@41": 1 } }),
  r(newer, "await#1", L(4, 2), { slots: { "new.callee@32": Thing, "new.arg@42": 1 } }),
  r(newSpread, "await#1", L(5, 2), { slots: { "new.callee@36": Thing, "new.args@45": [1] } }),
  r(member, "await#1", L(6, "x"), { slots: { t: $dump.t1, "member.object@53": $dump.t1 } }),
  r(key, "await#1", L(7, 2), {
    slots: {
      k: "z",
      "call.callee@45": JSON.stringify,
      "call.this@45": JSON,
      "object.literal@60": { a: 1 },
      "object.key@68": "z",
    },
  }),
  r(value, "await#1", L(8, 3), {
    slots: { "call.callee@32": JSON.stringify, "call.this@32": JSON, "object.literal@47": { a: 1 } },
  }),
  r(elements, "await#1", L(9, 3), { slots: { "array.element@36": 1, "array.element@39": 2 } }),
  r(hole, "await#1", L(10, 3), { slots: { "array.literal@31": [1, ,] } }),
  r(left, "await#1", L(11, 6), { slots: { a: 5, "binary.left@44": 5 } }),
  r(prop, "await#1", L(12, 1), { slots: { t: $dump.t2, "assign.object@38": $dump.t2 } }),
  r(index, "await#1", L(13, 2), {
    slots: { t: $dump.t3, k: "q", "assign.object@48": $dump.t3, "assign.key@48": "q" },
  }),
  r(compound, "await#1", L(14, 4), { slots: { n: 1, "assign.old@39": 1 } }),
  r(compoundProp, "await#1", L(15, 2), {
    slots: { t: $dump.t4, "assign.object@52": $dump.t4, "assign.old@52": 1 },
  }),
  r(template, "await#1", L(16, 2), {
    slots: { "template.part@35": "a", "template.part@39": "1", "template.part@40": "b" },
  }),
  r(Child.prototype.m, "await#1", L(17, "m"), {
    this: $dump.child,
    args: [7],
    slots: { x: 7, "super.this@24": $dump.child, "super.base@24": Base.prototype },
  }),
];
$dump.handle("main", $dump.resume(main, {
  at: "await#1",
  awaiting: Promise.all($dump.all),
  slots: { rs: $dump.TDZ },
}));
