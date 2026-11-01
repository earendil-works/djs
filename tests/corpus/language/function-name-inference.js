// difftest: stacks
// NamedEvaluation (0108, D2070): an anonymous function, arrow or class gets the binding's
// name only when it is the whole initializer, parenthesized or not; a call, member
// access or operator after it means it is only part of the initializer.
const out = [];
const show = (label, v) => out.push(label + "=" + JSON.stringify(v));
const n = (f) => (typeof f === "function" ? f.name : typeof f);

// const / let / var, in every shape.
const c1 = function () {}; const c2 = (function () {}); const c3 = ((() => 0));
const c4 = class {}; const c5 = (class {}); const c6 = async () => 0;
show("decl", [n(c1), n(c2), n(c3), n(c4), n(c5), n(c6)]);
const m1 = function () {}.name; const m2 = (function () {}).name; const m3 = (() => 0).name;
const m4 = (class {}).name; const m5 = class {}.name; const m6 = (async function () {})["name"];
show("member", [m1, m2, m3, m4, m5, m6]);
const k1 = (function () { return function () {}; })(); const k2 = (() => () => 0)();
const k3 = function () {}.bind(null); const k4 = (function () {}).call.bind(Function.prototype.call);
show("call", [n(k1), n(k2), n(k3), n(k4)]);
const o1 = (function () {}) || 0; const o2 = (() => 0) && (() => 1); const o3 = true ? function () {} : 0;
const o4 = (0, function () {}); const o5 = (function () {}) ?? 0; const o6 = (function () {})?.name;
show("operators", [n(o1), n(o2), n(o3), n(o4), n(o5), o6]);
let l1 = (function () {}), l2 = (function () {}).name; var v1 = (() => 0), v2 = (() => 0).length;
show("let/var", [n(l1), l2, n(v1), v2]);

// Assignment and logical assignment.
let a1, a2, a3, a4, a5;
a1 = (function () {}); a2 = (function () {}).name; a3 = function () {}.bind(null);
a4 ??= (() => 0); a5 ||= (() => 0).name;
show("assign", [n(a1), a2, n(a3), n(a4), a5]);

// Default parameters and destructuring defaults.
function params(p1 = (function () {}), p2 = (function () {}).name, p3 = (() => () => 0)()) {
  return [n(p1), p2, n(p3)];
}
show("params", params());
const { d1 = (function () {}), d2 = (function () {}).name } = {};
const [d3 = (() => 0), d4 = (() => 0)()] = [];
let d5, d6;
({ d5 = (class {}), d6 = (class {}).name } = {});
show("destructuring", [n(d1), d2, n(d3), d4, n(d5), d6]);

// Object literal values and class fields.
const obj = { p: (function () {}), q: (function () {}).name, r: (() => () => 0)(), s: function () {}.name };
show("object", [n(obj.p), obj.q, n(obj.r), obj.s]);
class Fields {
  f1 = (function () {});
  f2 = (function () {}).name;
  static f3 = (() => 0);
  static f4 = (() => () => 0)();
  #f5 = (class {});
  f5() { return n(this.#f5); }
}
const fi = new Fields();
show("fields", [n(fi.f1), fi.f2, n(Fields.f3), n(Fields.f4), fi.f5()]);
console.log(out.join("\n"));

// Stack frames of immediately called functions are anonymous.
const iife = (() => new Error("x").stack)();
console.log(iife);
const iife2 = (function () { return new Error("y").stack; })();
console.log(iife2);
