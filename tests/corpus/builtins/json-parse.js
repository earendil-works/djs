// JSON.parse: values, prototypes, errors with qjs messages, and the reviver (order,
// deletion, holder, nested replacement).
const v = JSON.parse('{"a":[1,2.5,-0,1e3,"s\\u0041\\n"],"b":{"c":null,"d":true,"e":false},"":0}');
console.log(JSON.stringify(v), Object.getPrototypeOf(v) === Object.prototype, Array.isArray(v.a), Object.is(v.a[2], -0));
console.log(JSON.parse(" 42 "), JSON.parse('"x"'), JSON.parse("null"), JSON.parse("[]").length, JSON.parse(new String("[7]"))[0]);
console.log(JSON.parse('{"__proto__":1}').__proto__, Object.keys(JSON.parse('{"b":1,"a":2,"1":3}')).join());
for (const t of ["{a", "[1,]", "", "01", "tru", '"\\x"', '{"a":1}x', "[1 2]", '"\u0001"', "{\"a\" 1}", undefined, "1e", "-", "[".repeat(3)]) {
  try { JSON.parse(t); console.log("parsed", t); } catch (e) { console.log(e.name, e.message); }
}
const log = [];
const r = JSON.parse('{"a":[1,{"b":2}],"c":3,"d":"x"}', function (k, val) {
  log.push(k + "=" + JSON.stringify(val) + (this === undefined ? "" : ":" + typeof this));
  if (k === "c") return undefined;
  if (typeof val === "number") return val * 10;
  return val;
});
console.log(log.join(" | "));
console.log(JSON.stringify(r), "c" in r);
console.log(JSON.parse("[1,2,3]", (k, val) => (k === "1" ? undefined : val)).length, 1 in JSON.parse("[1,2,3]", (k, val) => (k === "1" ? undefined : val)));
console.log(JSON.parse("5", function (k, val) { return [k, val, this[""]]; }).join());
console.log(JSON.parse('{"a":1}', 5).a, JSON.parse('{"a":1}', null).a);
const frozen = JSON.parse('{"a":{"b":1}}', function (k, val) { if (k === "a") Object.freeze(val); if (k === "b") return 2; return val; });
console.log(JSON.stringify(frozen));
try { JSON.parse("[1]", function () { throw new RangeError("boom"); }); } catch (e) { console.log(String(e)); }
console.log(JSON.parse({ toString() { return '{"t":1}'; } }).t, Object.prototype.toString.call(JSON), JSON.parse.length, JSON.parse.name);
