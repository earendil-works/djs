const rows = [];
for (let i = 0; i < 2000; i++) rows.push({ id: i, name: "item " + i, path: "/src/file_" + i + ".ts", size: i * 37, tags: ["a", "b"], ok: i % 2 === 0 });
let total = 0;
for (let r = 0; r < 20; r++) {
  const back = JSON.parse(JSON.stringify(rows));
  for (const row of back) total += row.size + row.name.length + (row.ok ? 1 : 0);
}
return total;
