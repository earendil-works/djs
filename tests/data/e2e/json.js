// JSON (0033): parse with and without a reviver, stringify on the fast path and the full
// algorithm (toJSON, replacers, gaps, wrappers, accessors), and their errors.
const text = '{"name":"djs","tags":["a","b"],"n":[1,2.5,-3],"nested":{"ok":true,"none":null}}';
const v = JSON.parse(text);
console.log(v.name, v.tags.length, v.nested.ok, JSON.stringify(v) === text);
console.log(JSON.stringify(JSON.parse(text, (k, x) => (typeof x === "number" ? x + 1 : x))));
console.log(JSON.stringify(v, ["name", "n"], 1));
console.log(JSON.stringify({ d: new Date(0), w: [new Number(1), new String("s"), new Boolean(true)], f() {} }));
console.log(JSON.stringify({ get g() { return { h: [1] }; } }, (k, x) => (k === "h" ? x.concat(2) : x), "\t"));
const big = [];
for (let i = 0; i < 200; i++) big.push({ i, s: "x" + i, toJSON: i === 150 ? () => "late" : undefined });
const out = JSON.stringify(big);
console.log(out.length, out.slice(0, 40), JSON.parse(out)[150]);
const c = []; c.push({ c });
for (const f of [() => JSON.parse("{bad"), () => JSON.stringify(c), () => JSON.stringify(c, (k, x) => x)]) {
  try { f(); } catch (e) { console.log(e.name, e.message); }
}
