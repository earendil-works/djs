// JSON.stringify of codemode-like records where one field is a Date (toJSON): the whole
// value leaves the leaf fast path (0090).
// bench: expect 2412260
const rows = [];
const when = new Date(Date.UTC(2024, 0, 2, 3, 4, 5));
for (let i = 0; i < 2000; i++) {
  rows.push({ id: i, name: "item " + i, path: "/src/file_" + i + ".ts", size: i * 37, tags: ["a", "b"], meta: { ok: i % 2 === 0, n: [i, i + 1] } });
}
rows[1000].when = when;
let total = 0;
for (let r = 0; r < 10; r++) total += JSON.stringify({ rows, at: when }).length;
return total;
