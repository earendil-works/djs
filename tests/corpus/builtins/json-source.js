// JSON.parse source text access: the reviver's context (source for primitives, kept only
// while the value is the parsed one), JSON.rawJSON and JSON.isRawJSON, stringify of raw
// JSON, and qjs's messages.
const log = [];
const v = JSON.parse('{"a": 1.50, "b": [true, null, "x\\u0041"], "c": {"d": -0, "e": 1e2}, "f": 2}', function (k, val, ctx) {
  log.push(JSON.stringify(k) + "=" + JSON.stringify(val) + " " + JSON.stringify(ctx));
  return val;
});
console.log(log.join("\n"));
console.log(JSON.stringify(v));
const log2 = [];
let once = true;
JSON.parse('[1, [], {"p": 2}]', function (k, val, { source }) {
  log2.push(k + ":" + source);
  if (once) { once = false; this[1].push("added"); this[2].p = 3; }
  return this[k];
});
console.log(log2.join(" "));
const big = JSON.parse("12345678901234567890", (k, val, { source }) => [val, source]);
console.log(JSON.stringify(big), JSON.parse("[]", (k, v, c) => Object.keys(c).length), JSON.parse("1", function (k, v, c) { return Object.getPrototypeOf(c) === Object.prototype; }));
for (const t of ["1", "-1.5e3", "null", "true", '"s\\n"', "", " 1", "1 ", "{}", "[1]", "x", '"a', "\t2"]) {
  try {
    const r = JSON.rawJSON(t);
    console.log(JSON.stringify(t), r.rawJSON, Object.isFrozen(r), Object.getPrototypeOf(r), JSON.isRawJSON(r), Object.keys(r).join());
  } catch (e) { console.log(JSON.stringify(t), e.name, e.message); }
}
console.log(JSON.stringify({ a: JSON.rawJSON("1e1000"), b: [JSON.rawJSON('"x"'), JSON.rawJSON("12345678901234567890")] }));
console.log(JSON.stringify(JSON.rawJSON("7")), JSON.stringify([JSON.rawJSON("1")], null, 2), JSON.stringify({ x: 1 }, (k, val) => (k === "x" ? JSON.rawJSON("99") : val)));
console.log(JSON.isRawJSON({}), JSON.isRawJSON(1), JSON.isRawJSON(Object.freeze(Object.assign(Object.create(null), { rawJSON: "1" }))), Object.prototype.toString.call(JSON.rawJSON(1)));
console.log(JSON.rawJSON.length, JSON.isRawJSON.length, JSON.rawJSON.name, typeof JSON.rawJSON(1).rawJSON);
