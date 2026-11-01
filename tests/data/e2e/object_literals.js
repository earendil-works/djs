const base = { greet() { return "hi " + this.name; } };
const k = "comp";
let log = "";
const o = {
  __proto__: base,
  name: "o",
  m(x) { return x * 2; },
  get g() { return this.name + "!"; },
  set s(v) { log += v; },
  [k + "uted"]() { return 1; },
  get [k]() { return "computed getter"; },
  ["arrow" + 1]: () => 2,
};
o.s = "a"; o.s = "b";
console.log(o.greet(), o.m(21), o.g, log, o.computed(), o.comp, o.arrow1());
console.log(o.m.name, o.computed.name, o.arrow1.name, typeof o.m.prototype);
const acc = { get both() { return 1; }, set both(v) {} };
console.log(acc.both);
