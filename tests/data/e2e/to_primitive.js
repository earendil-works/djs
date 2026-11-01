const v = { valueOf: function () { return 42; } };
const s = { toString: function () { return "str"; } };
const both = { valueOf: function () { return 1; }, toString: function () { return "two"; } };
console.log(v + 1, v * 2, -v, v > 41, 41 < v, v == 42, v === 42, `${s}`, s + "!", both + "", `${both}`);
const o = {}; o[s] = 1; console.log(o.str, s in o, delete o[s], "str" in o);
let n = { valueOf: function () { return 5; } }; n++; console.log(n);
let order = "";
const a = { valueOf: function () { order += "a"; return 1; } };
const b = { valueOf: function () { order += "b"; return 2; } };
console.log(a + b, a < b, b > a, order);
const bad = { valueOf: function () { return {}; }, toString: function () { return {}; } };
try { bad + 1; } catch (e) { console.log(e.name, e.message); }
const thrower = { valueOf: function () { throw "inside"; } };
try { thrower * 2; } catch (e) { console.log("caught", e); }
