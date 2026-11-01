// JSON round trips of codemode-like records.
// bench: expect 1953680
const rows = [];
for (let i = 0; i < 2000; i++) {
  rows.push({ id: i, name: "item " + i, path: "/src/file_" + i + ".ts", size: i * 37, tags: ["a", "b"], ok: i % 2 === 0 });
}
let total = 0;
for (let r = 0; r < 10; r++) {
  const text = JSON.stringify(rows);
  const back = JSON.parse(text);
  total += text.length + back.length;
}
return total;
