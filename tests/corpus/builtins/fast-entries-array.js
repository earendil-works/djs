// The @fast entries of 0091 (D1910): Array element loops, String.fromCharCode and the
// URI functions. Each case also takes the declining path (objects with valueOf,
// non-arrays, holes, errors), which must behave the same.
const out = [];
const log = (...a) => out.push(a.map(String).join(" "));
const order = [];
const obj = (v) => ({ valueOf() { order.push(v); return v; } });
{
  const a = [1, 2, 3, 2, 1];
  log("idx", a.indexOf(2), a.indexOf(2, 2), a.indexOf(2, -2), a.indexOf(2, Infinity), a.indexOf(2, -Infinity), a.indexOf(2, obj(3)), a.indexOf(2, "1"), a.indexOf(2, null), a.indexOf(9));
  log("last", a.lastIndexOf(2), a.lastIndexOf(2, 2), a.lastIndexOf(2, -3), a.lastIndexOf(2, -Infinity), a.lastIndexOf(2, Infinity), a.lastIndexOf(2, obj(1)), a.lastIndexOf(2, undefined), [].lastIndexOf(1));
  log("inc", a.includes(3), a.includes(3, 3), a.includes(3, -3), a.includes(3, Infinity), a.includes(3, obj(0)), [NaN].includes(NaN), [-0].includes(0), [].includes(undefined));
  log("slice", a.slice(1, 3), a.slice(-2), a.slice(3, 1).length, a.slice(obj(1), obj(2)), a.slice("1", "x").length, a.slice(1.5, 3.9));
  log("join", a.join(), a.join("-"), a.join(1), a.join(null), a.join(true), a.join(obj(0)), [[1, 2], 3].join("|"), ["a", , "b"].join());
  try { a.join(Symbol()); } catch (e) { log("joinsym", e.constructor.name); }
  log("fill", [1, 2, 3].fill(0), [1, 2, 3].fill(9, 1), [1, 2, 3].fill(9, obj(-1)), [1, 2, 3].fill(7, 1, 2), [].fill(1).length);
  const p = [1];
  log("push", p.push(2), p.push(), p.push(3, 4), p.push(obj(5)), p.length, p[4] instanceof Object);
  log("pop", p.pop() instanceof Object, p.pop(), [].pop(), p.length);
  log("this", Array.prototype.indexOf.call("abc", "b"), Array.prototype.join.call({ length: 2, 0: "x", 1: "y" }), Array.prototype.slice.call("abc", 1).join(""), Array.prototype.push.call({ length: 0 }, 1));
  log("order", order.join());
}
{
  log("fcc", String.fromCharCode(), String.fromCharCode(65, 66), String.fromCharCode(0x10041, -1, 65.9), String.fromCharCode("67", true, null), String.fromCharCode(0xd83d, 0xde00).length, String.fromCharCode(1, 2, 3, 4, 5, 6, 7, 8).length, String.fromCharCode(obj(68)));
  try { String.fromCharCode(Symbol()); } catch (e) { log("fccsym", e.constructor.name); }
  log("uri", decodeURI("%41%E2%82%AC%23"), decodeURIComponent("%41%E2%82%AC%23"), encodeURI("a b€#"), encodeURIComponent("a b€#"), decodeURI(12), encodeURI(undefined), encodeURIComponent(null));
  for (const [f, x] of [[decodeURI, "%E2%82"], [decodeURIComponent, "%zz"], [encodeURI, "\uD800"], [encodeURIComponent, "\uDFFF"]]) {
    try { f(x); log("no error", f.name); } catch (e) { log("urierr", f.name, e.constructor.name, e.message); }
  }
  log("uriobj", decodeURI({ toString() { return "%41"; } }), encodeURIComponent(obj(5)));
}
console.log(out.join("\n"));
