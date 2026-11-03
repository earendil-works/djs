// JSON.stringify results past 4 KB (D2553): the output moves into the result string at
// check points, long strings go out in parts, and a rope's flattening in between starts a
// new piece. Printed as lengths, slices around the joins and a parse round trip (no
// per-unit loops in JS: the case runs under --suspend-every 1).
function show(label, s) {
  const back = JSON.stringify(JSON.parse(s)) === s;
  console.log(label, s.length, back, JSON.stringify(s.slice(4090, 4110)), JSON.stringify(s.slice(8186, 8200)), JSON.stringify(s.slice(-24)));
}
const rows = [];
for (let i = 0; i < 600; i++) rows.push({ id: i, name: "item " + i, ok: i % 3 === 0, n: [i, i / 4] });
show("rows", JSON.stringify(rows));
show("rows indented", JSON.stringify(rows.slice(0, 150), null, 2));
// A wide unit after the output was already moved out (narrow first, then wide).
const late = rows.slice(0, 150).concat(["caf\u00e9 \u4e2d\u6587 \ud83d\ude00"]);
show("widens late", JSON.stringify(late));
// Long strings: escapes, a surrogate pair across the 4096-unit split, a lone half.
const long = "a\"b\\c\n".repeat(1500);
show("long", JSON.stringify({ long }));
const pair = "x".repeat(4095) + "\ud83d\ude00" + "y".repeat(5000) + "\ud800" + "z";
show("pair", JSON.stringify([pair]));
const narrowLong = "\u00e9".repeat(9000);
show("latin1 long", JSON.stringify(narrowLong));
// Ropes flattened while the output is being written.
const parts = [];
for (let i = 0; i < 40; i++) parts.push("part " + i + ": " + "q".repeat(200) + String(i));
show("ropes", JSON.stringify(parts.concat(parts.map((p, i) => p + "/" + i))));
// The full algorithm with large plain subtrees (replacer, toJSON).
show("replacer", JSON.stringify({ rows: rows.slice(0, 200) }, (k, v) => (k === "ok" ? undefined : v)));
show("toJSON", JSON.stringify({ toJSON() { return rows.slice(0, 250); } }));
console.log(JSON.parse(JSON.stringify(rows)).length, JSON.stringify(rows) === JSON.stringify(rows));
