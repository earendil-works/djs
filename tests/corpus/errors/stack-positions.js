// difftest: stacks
// Call-site positions in stack traces (0105, D2031): qjs reports a call that a plain
// `return` follows (a tail call) at the `return`, every other call at its `(`, and
// `new X(...)` at the token after `X`.
function thrower() { throw new Error("t"); }
function Thrower() { thrower(); }
const o = { m() { return thrower(); }, n: { k() { thrower(); } }, get g() { return thrower(); } };
class P { m() { return thrower(); } }
class Q extends P { m() { return super.m(); } }
class K { constructor() { return thrower(); } }
class K2 extends Object { constructor() { super(); return thrower(); } }
class S { constructor() { thrower(); } }
class T extends S { constructor() { super(); } }
const cases = {
  plain() { return thrower(); },
  local() { const x = thrower(); return x; },
  statement() { thrower(); },
  operand() { return 1 + thrower(); },
  method() { return o.m(); },
  nested() { return o.n.k(); },
  computed() { return o["m"](); },
  getter() { return o.g; },
  superCall() { return new Q().m(); },
  multiLine() { return thrower(1,
    2); },
  paren() { return (thrower()); },
  comma() { return (1, thrower()); },
  commaFirst() { return thrower(), 1; },
  indirect() { return (0, thrower)(); },
  viaCall() { return thrower.call(null); },
  spread(...a) { return thrower(...a); },
  optional() { return thrower?.(); },
  optionalMember() { return o?.m(); },
  conditional() { return true ? thrower() : 0; },
  conditionalElse() { return false ? 0 : thrower(); },
  logical() { return thrower() || 1; },
  tagged() { return thrower`x`; },
  template() { return `${thrower()}`; },
  voided() { return void thrower(); },
  ifReturn() { if (true) return thrower(); },
  ownLine() {
    return thrower();
  },
  inTry() { try { return thrower(); } catch (e) { throw e; } },
  inCatch() { try { throw 1; } catch (e) { return thrower(); } },
  inFinally() { try { } finally { return thrower(); } },
  afterTry() { try { } catch (e) { } return thrower(); },
  forOf() { for (const x of [1]) { return thrower(); } },
  forIn() { for (const x in { a: 1 }) { return thrower(); } },
  switched() { switch (1) { case 1: return thrower(); } },
  labeled() { lbl: { return thrower(); } },
  block() { { let z = 1; const f = () => z; return thrower(); } },
  using() { using r = null; return thrower(); },
  arrowBody() { return (() => { return thrower(); })(); },
  arrowExpr() { return (() => thrower())(); },
  functionExpr() { return (function () { return thrower(); })(); },
  generator() { return (function* () { return thrower(); })().next(); },
  callback() { return [1].map((x) => thrower()); },
  newCall() { return new Thrower(); },
  newBare() { return new Thrower; },
  newStatement() { new Thrower(); },
  newLocal() { const x = new Thrower(); return x; },
  newMultiLine() { return new
    Thrower; },
  baseCtor() { return new K(); },
  derivedCtor() { return new K2(); },
  superCtor() { return new T(); },
};
for (const [name, f] of Object.entries(cases)) {
  try {
    f();
  } catch (e) {
    console.log(name + ":\n" + e.stack);
  }
}
async function asyncReturn() { return thrower(); }
try { await asyncReturn(); } catch (e) { console.log("async:\n" + e.stack); }
