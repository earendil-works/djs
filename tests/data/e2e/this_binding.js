// mode: body
const o = { v: 1 };
function plain() { return this; }
console.log(typeof plain(), o.v);
const obj = { n: 41, f: function () { return this.n + 1; } };
console.log(obj.f());
