"djs-dump 2";
$dump.keep = (o, keys) => {
  for (const k of Reflect.ownKeys(o)) {
    if (!["length", "name", "prototype", "constructor"].includes(k) && !keys.includes(k)) delete o[k];
  }
  for (const k of keys) if (!Object.prototype.hasOwnProperty.call(o, k)) throw new TypeError("the class does not make " + String(k));
};
$dump.p1 = $dump.pending(1, "later", ["m"]);
class Base { m(x) { return "base " + x; } }
let Child = ((Base) => class Child extends Base {
  async m(x) { return super[await later("m")](x); }
})(Base);
$dump.n2 = { __proto__: Child.prototype };
$dump.a3 = $dump.resume(Child.prototype.m, { at: "await#1", awaiting: $dump.p1, this: $dump.n2, args: [7], bindings: { x: 7 }, operands: [["super.this", $dump.n2], ["super.base", Base.prototype]] });
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
$dump.keep(Base, []);
$dump.keep(Base.prototype, ["m"]);
$dump.keep(Child, []);
$dump.keep(Child.prototype, ["m"]);
